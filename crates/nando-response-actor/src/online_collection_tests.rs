use super::*;
use serde_json::json;

fn legacy_default_config() -> OnlineCollectionConfig {
    OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        ..OnlineCollectionConfig::default()
    }
}

#[test]
fn mixed_support_blockers_trigger_program_subcenter_split() {
    for blocker in [
        "support_program_cover_empty",
        "support_program_cover_incomplete",
        "support_consensus_authority_unproven",
    ] {
        assert!(support_blocker_requires_subcenter_split(Some(blocker)));
    }
    assert!(!support_blocker_requires_subcenter_split(Some(
        "support_rows_below_32"
    )));
    assert!(!support_blocker_requires_subcenter_split(None));
}

#[test]
fn teacher_mismatch_never_attaches_empty_program_receipt() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-teacher-mismatch-{}-{}",
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
        .observe_replay_training_buffered(observation(1, "3"))
        .expect("initial support");

    let mismatch = observation(2, "4");
    assert!(
        miner
            .matching_unfrozen_buckets(&mismatch)
            .expect("matching buckets")
            .is_empty()
    );
    miner
        .observe_replay_training_buffered(mismatch)
        .expect("mismatched teacher observation");
    assert!(miner.checkpoint.buckets.iter().all(|bucket| {
        bucket
            .support
            .iter()
            .all(|receipt| !receipt.matched_program_sha256.is_empty())
    }));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn unfrozen_matching_is_bounded_by_wave_route_budget() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-bounded-route-{}-{}",
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
        max_buckets: 32,
        max_receipts_per_bucket: 64,
    };
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
    let matching_observation = observation(1, "3");
    miner
        .observe_replay_training_buffered(matching_observation.clone())
        .expect("initial support");
    let template = miner.checkpoint.buckets[0].clone();
    let program = template.programs.values().next().expect("program").clone();
    miner.checkpoint.buckets.clear();
    for bucket_index in 0..20 {
        let mut bucket = template.clone();
        bucket.bucket_id = format!("{bucket_index:064x}");
        bucket.programs = (0..16)
            .map(|program_index| {
                (
                    format!("{bucket_index:032x}{program_index:032x}"),
                    program.clone(),
                )
            })
            .collect();
        miner.checkpoint.buckets.push(bucket);
    }

    let checks_before = miner.checkpoint.version_space_intersection_checks_total;
    let scheduled_before = miner.checkpoint.guard_scheduled_buckets_total;
    let pruned_before = miner.checkpoint.guard_pruned_buckets_total;
    let matching = miner
        .matching_unfrozen_buckets(&matching_observation)
        .expect("matching buckets");

    assert_eq!(matching.len(), MAX_UNFROZEN_ROUTE_BUCKETS);
    assert_eq!(
        miner.checkpoint.guard_scheduled_buckets_total - scheduled_before,
        MAX_UNFROZEN_ROUTE_BUCKETS as u64
    );
    assert_eq!(
        miner.checkpoint.guard_pruned_buckets_total - pruned_before,
        12
    );
    assert_eq!(
        miner.checkpoint.version_space_intersection_checks_total - checks_before,
        (MAX_UNFROZEN_ROUTE_BUCKETS * MAX_UNFROZEN_ROUTE_PROGRAMS) as u64
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn support_program_subcenter_survives_restart_without_parent_remerge() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-subcenter-restart-{}-{}",
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
    let examples = (1..=4)
        .map(|index| observation(index, "3"))
        .collect::<Vec<_>>();
    let program = enumerate_source_neutral_response_programs(&examples[0].example)
        .expect("version space")
        .programs
        .into_iter()
        .find(|program| {
            is_source_neutral_response_program(program)
                && examples.iter().all(|example| {
                    independently_verified_authority_response(program, &example.example).as_deref()
                        == Some("3")
                })
        })
        .expect("portable count program");
    let program_sha256 = canonical_json_sha256(&program).expect("program digest");
    let programs = BTreeMap::from([(program_sha256.clone(), program)]);
    let support = examples
        .iter()
        .map(|example| {
            receipt_with_program_atoms(example, true, &programs).expect("support receipt")
        })
        .collect::<Vec<_>>();
    let parent = OnlineCollectionBucket {
        bucket_id: "a".repeat(64),
        archetype_id: "b".repeat(64),
        programs,
        common_request_atom_ids: BTreeSet::new(),
        support,
        future: Vec::new(),
        runtime_examples: BTreeMap::new(),
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };
    let subcenter = support_program_subcenters(&parent, 4, 16)
        .expect("subcenters")
        .into_iter()
        .next()
        .expect("program subcenter");
    assert_ne!(parent.archetype_id, subcenter.archetype_id);
    let subcenter_archetype_id = subcenter.archetype_id.clone();

    let path = root.join("checkpoint.cbor");
    let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
    miner.checkpoint.buckets = vec![parent, subcenter];
    miner.persist().expect("persist parent and subcenter");
    let checkpoint_before = fs::read(&path).expect("checkpoint before reopen");
    drop(miner);

    let reopened = OnlineCollectionMiner::open(&path, config).expect("reopen");
    let checkpoint_after = fs::read(&path).expect("checkpoint after reopen");
    assert_eq!(checkpoint_before, checkpoint_after);
    assert_eq!(reopened.checkpoint.buckets.len(), 2);
    assert!(
        reopened
            .checkpoint
            .buckets
            .iter()
            .any(|bucket| bucket.archetype_id == subcenter_archetype_id)
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn renderer_variants_form_one_law_subcenter_and_survive_restart() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-law-subcenter-restart-{}-{}",
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
    let examples = (1_usize..=4)
        .map(|index| {
            let rendered = !index.is_multiple_of(2);
            let mut observation = observation(index, if rendered { "Total: 3." } else { "3" });
            observation.example.provider_payload["input"][0]["content"][0]["text"] =
                Value::String(if rendered {
                    "Return exactly \"Total: {count}.\"".to_owned()
                } else {
                    "Return direct count".to_owned()
                });
            observation
        })
        .collect::<Vec<_>>();
    let direct = ResponseProgram::compose_collection(
        vec![
            crate::CollectionProgramStep::SelectOnlyArrayField,
            crate::CollectionProgramStep::Count,
        ],
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    let rendered =
        direct
            .clone()
            .with_collection_renderer(crate::CollectionOutputRenderer::RequestTemplate {
                marker: crate::RequestTemplateMarker::BracedCount,
            });
    assert_eq!(
        response_law_key(&direct).expect("direct law"),
        response_law_key(&rendered).expect("rendered law")
    );
    let direct_sha256 = canonical_json_sha256(&direct).expect("direct digest");
    let rendered_sha256 = canonical_json_sha256(&rendered).expect("rendered digest");
    for (index, example) in examples.iter().enumerate() {
        let program = if (index + 1).is_multiple_of(2) {
            &direct
        } else {
            &rendered
        };
        assert_eq!(
            independently_verified_authority_response_result(program, &example.example),
            Ok(example.example.expected_response.clone()),
            "concrete adapter {}",
            index + 1
        );
    }
    let programs = BTreeMap::from([
        (direct_sha256.clone(), direct),
        (rendered_sha256.clone(), rendered),
    ]);
    let runtime_examples = examples
        .iter()
        .map(|example| {
            (
                example.evidence_graph_sha256.clone(),
                example.example.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let support = examples
        .iter()
        .enumerate()
        .map(|(index, example)| {
            let mut receipt =
                receipt_with_program_atoms(example, true, &programs).expect("support receipt");
            receipt.matched_program_sha256 = vec![if (index + 1).is_multiple_of(2) {
                direct_sha256.clone()
            } else {
                rendered_sha256.clone()
            }];
            receipt
        })
        .collect::<Vec<_>>();
    let parent = OnlineCollectionBucket {
        bucket_id: "c".repeat(64),
        archetype_id: "d".repeat(64),
        programs,
        common_request_atom_ids: BTreeSet::new(),
        support,
        future: Vec::new(),
        runtime_examples,
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };
    let law_child = support_law_subcenters(&parent, 4, 16)
        .expect("law subcenters")
        .into_iter()
        .next()
        .expect("law subcenter");
    assert_eq!(law_child.support.len(), 4);
    assert_eq!(law_child.programs.len(), 2);
    assert!(law_child.programs.values().any(|program| matches!(
        &program.operation,
        crate::ResponseOperation::ComposeCollection {
            renderer: crate::CollectionOutputRenderer::Direct,
            ..
        }
    )));
    assert!(law_child.programs.values().any(|program| matches!(
        &program.operation,
        crate::ResponseOperation::ComposeCollection {
            renderer: crate::CollectionOutputRenderer::RequestTemplate { .. },
            ..
        }
    )));
    let law_archetype_id = law_child.archetype_id.clone();

    let path = root.join("checkpoint.cbor");
    let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
    miner.checkpoint.buckets = vec![parent, law_child];
    miner.maybe_freeze(1).expect("freeze law child");
    assert!(miner.checkpoint.buckets[0].frozen_program_sha256.is_none());
    assert!(miner.checkpoint.buckets[1].frozen_program_sha256.is_some());
    miner
        .persist()
        .expect("persist parent and frozen law child");
    drop(miner);

    let reopened = OnlineCollectionMiner::open(&path, config).expect("reopen");
    assert_eq!(reopened.checkpoint.buckets.len(), 2);
    assert!(reopened.checkpoint.buckets.iter().any(|bucket| {
        bucket.archetype_id == law_archetype_id && bucket.frozen_program_sha256.is_some()
    }));
    assert!(
        reopened
            .checkpoint
            .buckets
            .iter()
            .any(|bucket| bucket.bucket_id == "c".repeat(64))
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn maximal_decidable_subcenter_keeps_32_clean_rows_and_excludes_ambiguous_layout() {
    let alpha = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::JsonField {
            field: "alpha".to_owned(),
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    let beta = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::JsonField {
            field: "beta".to_owned(),
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    assert_eq!(
        response_law_key(&alpha).expect("alpha law"),
        response_law_key(&beta).expect("beta law")
    );
    let alpha_digest = canonical_json_sha256(&alpha).expect("alpha digest");
    let beta_digest = canonical_json_sha256(&beta).expect("beta digest");
    let programs = BTreeMap::from([(alpha_digest.clone(), alpha), (beta_digest.clone(), beta)]);
    let clean_layouts = ["a".repeat(64), "b".repeat(64)];
    let ambiguous_layout = "c".repeat(64);
    let support = (0..36)
        .map(|index| OnlineCollectionReceipt {
            evidence_graph_sha256: format!("{:064x}", index + 1),
            client_intent_id_sha256: format!("{:064x}", index + 1_000),
            session_id_sha256: format!("{:064x}", index % 8 + 2_000),
            event_time_unix_nanos: Some(index as u64 + 1),
            layout_sha256: if index < 32 {
                clean_layouts[index / 16].clone()
            } else {
                ambiguous_layout.clone()
            },
            estimated_input_tokens: 100,
            verifier_pass: true,
            request_atom_ids: vec![7, 11],
            matched_program_sha256: vec![if index < 32 || index.is_multiple_of(2) {
                alpha_digest.clone()
            } else {
                beta_digest.clone()
            }],
            witness_class_commitment_sha256: None,
            witness_round: None,
            witness_candidates_before: None,
            witness_candidates_after: None,
        })
        .collect::<Vec<_>>();
    let parent = OnlineCollectionBucket {
        bucket_id: "d".repeat(64),
        archetype_id: "e".repeat(64),
        programs,
        common_request_atom_ids: BTreeSet::from([7, 11]),
        support,
        future: Vec::new(),
        runtime_examples: BTreeMap::new(),
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };
    assert!(matches!(
        support_consensus_candidate(&parent).expect("parent consensus"),
        SupportConsensusCandidate::Blocked("support_phase_adapter_unproven")
    ));

    let child = maximal_decidable_support_subcenter(&parent, 32, 128)
        .expect("decidable split")
        .expect("clean child");
    assert_eq!(child.support.len(), 32);
    assert!(
        child
            .support
            .iter()
            .all(|receipt| receipt.layout_sha256 != ambiguous_layout)
    );
    assert_eq!(
        child
            .support
            .iter()
            .map(|receipt| receipt.layout_sha256.clone())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(clean_layouts)
    );
    assert!(matches!(
        support_consensus_candidate(&child).expect("child consensus"),
        SupportConsensusCandidate::Ready(_)
    ));
    assert_eq!(child.wrong_accepts, 0);
}

#[test]
fn durable_pre_action_atoms_restore_phase_adapter_without_raw_examples() {
    let alpha = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::JsonField {
            field: "alpha".to_owned(),
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    let beta = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::JsonField {
            field: "beta".to_owned(),
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    let alpha_digest = canonical_json_sha256(&alpha).expect("alpha digest");
    let beta_digest = canonical_json_sha256(&beta).expect("beta digest");
    let programs = BTreeMap::from([(alpha_digest.clone(), alpha), (beta_digest.clone(), beta)]);
    let alpha_atom = crate::stable_atom_id("request:select-alpha");
    let beta_atom = crate::stable_atom_id("request:select-beta");
    let program_atoms = programs
        .values()
        .flat_map(response_program_required_routing_atom_ids)
        .collect::<BTreeSet<_>>();
    let support = (0..32)
        .map(|index| {
            let mut request_atom_ids = program_atoms.iter().copied().collect::<Vec<_>>();
            request_atom_ids.push(if index < 16 { alpha_atom } else { beta_atom });
            request_atom_ids.sort_unstable();
            request_atom_ids.dedup();
            OnlineCollectionReceipt {
                evidence_graph_sha256: format!("{:064x}", index + 40_000),
                client_intent_id_sha256: format!("{:064x}", index + 50_000),
                session_id_sha256: format!("{:064x}", index % 8 + 60_000),
                event_time_unix_nanos: Some(index as u64 + 1),
                layout_sha256: "a".repeat(64),
                estimated_input_tokens: 100,
                verifier_pass: true,
                request_atom_ids,
                matched_program_sha256: vec![if index < 16 {
                    alpha_digest.clone()
                } else {
                    beta_digest.clone()
                }],
                witness_class_commitment_sha256: None,
                witness_round: None,
                witness_candidates_before: None,
                witness_candidates_after: None,
            }
        })
        .collect();
    let bucket = OnlineCollectionBucket {
        bucket_id: "d".repeat(64),
        archetype_id: "e".repeat(64),
        programs,
        common_request_atom_ids: BTreeSet::new(),
        support,
        future: Vec::new(),
        runtime_examples: BTreeMap::new(),
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };

    let candidate = match support_consensus_candidate(&bucket).expect("consensus") {
        SupportConsensusCandidate::Ready(candidate) => candidate,
        SupportConsensusCandidate::Blocked(reason) => panic!("blocked: {reason}"),
    };
    let crate::ResponseOperation::UniqueConsensus { variants, .. } = candidate.operation else {
        panic!("expected guarded consensus");
    };
    assert_eq!(variants.len(), 2);
    assert!(variants.iter().all(|variant| {
        variant.required_request_atom_ids == vec![alpha_atom]
            || variant.required_request_atom_ids == vec![beta_atom]
    }));

    let root = std::env::temp_dir().join(format!(
        "nando-durable-phase-migration-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("root");
    let path = root.join("collection.checkpoint");
    let config = legacy_default_config();
    let mut legacy = OnlineCollectionMiner::open(&path, config).expect("legacy shell");
    legacy.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V31;
    legacy.checkpoint.buckets = vec![bucket];
    legacy.persist().expect("persist v31");
    drop(legacy);

    let migrated = OnlineCollectionMiner::open(&path, config).expect("migrate v32");
    assert_eq!(
        migrated.checkpoint.pooling_strategy_version,
        ONLINE_COLLECTION_POOLING_STRATEGY_V36
    );
    assert_eq!(migrated.status().frozen_buckets_total, 1);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn durable_law_subcenter_restores_verified_rows_without_raw_examples() {
    let plain = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::JsonField {
            field: "value".to_owned(),
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    let json = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::JsonField {
            field: "value".to_owned(),
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::CanonicalJson,
        "completed",
    );
    let plain_digest = canonical_json_sha256(&plain).expect("plain digest");
    let json_digest = canonical_json_sha256(&json).expect("json digest");
    let support = (0..60)
        .map(|index| OnlineCollectionReceipt {
            evidence_graph_sha256: format!("{:064x}", index + 70_000),
            client_intent_id_sha256: format!("{:064x}", index + 80_000),
            session_id_sha256: format!("{:064x}", index % 8 + 90_000),
            event_time_unix_nanos: Some(index as u64 + 1),
            layout_sha256: "a".repeat(64),
            estimated_input_tokens: 100,
            verifier_pass: true,
            request_atom_ids: vec![crate::stable_atom_id("request:project")],
            matched_program_sha256: vec![if index < 40 {
                plain_digest.clone()
            } else {
                json_digest.clone()
            }],
            witness_class_commitment_sha256: None,
            witness_round: None,
            witness_candidates_before: None,
            witness_candidates_after: None,
        })
        .collect();
    let bucket = OnlineCollectionBucket {
        bucket_id: "1".repeat(64),
        archetype_id: "2".repeat(64),
        programs: BTreeMap::from([(plain_digest.clone(), plain), (json_digest, json)]),
        common_request_atom_ids: BTreeSet::new(),
        support,
        future: Vec::new(),
        runtime_examples: BTreeMap::new(),
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };

    let subcenters = support_law_subcenters(&bucket, 32, 128).expect("law subcenters");
    assert_eq!(subcenters.len(), 1);
    assert_eq!(subcenters[0].support.len(), 40);
    assert_eq!(subcenters[0].programs.len(), 1);
    assert!(subcenters[0].programs.contains_key(&plain_digest));
    assert!(
        support_law_subcenters(&subcenters[0], 32, 128)
            .expect("no recursive law subcenter")
            .is_empty()
    );

    let root = std::env::temp_dir().join(format!(
        "nando-exact-subcenter-dedup-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("root");
    let path = root.join("collection.checkpoint");
    let config = legacy_default_config();
    let mut left = subcenters[0].clone();
    left.bucket_id = "3".repeat(64);
    left.archetype_id = "4".repeat(64);
    let mut right = left.clone();
    right.bucket_id = "5".repeat(64);
    right.archetype_id = "6".repeat(64);
    let mut legacy = OnlineCollectionMiner::open(&path, config).expect("v33 shell");
    legacy.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V33;
    legacy.checkpoint.buckets = vec![right, left];
    legacy.persist().expect("persist duplicate v33 children");
    drop(legacy);

    let migrated = OnlineCollectionMiner::open(&path, config).expect("migrate v34");
    assert_eq!(
        migrated.checkpoint.pooling_strategy_version,
        ONLINE_COLLECTION_POOLING_STRATEGY_V36
    );
    assert_eq!(migrated.checkpoint.buckets.len(), 1);
    assert_eq!(migrated.checkpoint.buckets[0].bucket_id, "3".repeat(64));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn clean_pre_action_atoms_recover_32_rows_from_one_ambiguous_layout() {
    let alpha = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::JsonField {
            field: "alpha".to_owned(),
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    let beta = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::JsonField {
            field: "beta".to_owned(),
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    let alpha_digest = canonical_json_sha256(&alpha).expect("alpha digest");
    let beta_digest = canonical_json_sha256(&beta).expect("beta digest");
    let programs = BTreeMap::from([(alpha_digest.clone(), alpha), (beta_digest.clone(), beta)]);
    let observations = (0..42)
        .map(|index| OnlineCollectionObservation {
            evidence_graph_sha256: format!("{:064x}", index + 10_000),
            client_intent_id_sha256: format!("{:064x}", index + 20_000),
            session_id_sha256: format!("{:064x}", index % 8 + 30_000),
            event_time_unix_nanos: Some(index as u64 + 1),
            estimated_input_tokens: 100,
            example: CollectionSynthesisExample {
                provider_payload: json!({
                    "input": [
                        {"type":"message", "role":"user", "content":[{
                            "type":"input_text",
                            "text": if index < 32 { "select alpha" } else { "select value" }
                        }]},
                        {"type":"function_call_output", "output":"{\"alpha\":7,\"beta\":8}"}
                    ]
                }),
                expected_response: if index < 36 { "7" } else { "8" }.to_owned(),
            },
        })
        .collect::<Vec<_>>();
    let layout =
        structural_layout_sha256(&observations[0].example.provider_payload).expect("shared layout");
    assert!(observations.iter().all(|observation| {
        structural_layout_sha256(&observation.example.provider_payload).as_deref()
            == Ok(layout.as_str())
    }));
    let support = observations
        .iter()
        .enumerate()
        .map(|(index, observation)| {
            let mut value = receipt(observation, true).expect("receipt");
            value.matched_program_sha256 = vec![if index < 36 {
                alpha_digest.clone()
            } else {
                beta_digest.clone()
            }];
            value
        })
        .collect::<Vec<_>>();
    let runtime_examples = observations
        .iter()
        .map(|observation| {
            (
                observation.evidence_graph_sha256.clone(),
                observation.example.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let parent = OnlineCollectionBucket {
        bucket_id: "8".repeat(64),
        archetype_id: "9".repeat(64),
        programs,
        common_request_atom_ids: BTreeSet::new(),
        support,
        future: Vec::new(),
        runtime_examples,
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };
    assert!(matches!(
        support_consensus_candidate(&parent).expect("parent consensus"),
        SupportConsensusCandidate::Blocked("support_phase_adapter_unproven")
    ));

    let child = clean_pre_action_program_subcenter(&parent, 32, 128)
        .expect("pre-action split")
        .expect("clean child");
    assert_eq!(child.support.len(), 32);
    assert_eq!(child.programs.len(), 1);
    assert!(child.programs.contains_key(&alpha_digest));
    assert!(!child.common_request_atom_ids.is_empty());
    assert!(matches!(
        support_consensus_candidate(&child).expect("child consensus"),
        SupportConsensusCandidate::Ready(_)
    ));
    assert_eq!(child.wrong_accepts, 0);
}

#[test]
fn runtime_example_compaction_preserves_independent_authority() {
    let mut example = observation(1, "3");
    let program = enumerate_source_neutral_response_programs(&example.example)
        .expect("version space")
        .programs
        .into_iter()
        .find(|program| {
            is_source_neutral_response_program(program)
                && program_any(program, is_count_operation)
                && independently_verified_authority_response(program, &example.example).as_deref()
                    == Some("3")
        })
        .expect("count program");
    let input = example.example.provider_payload["input"]
        .as_array_mut()
        .expect("input");
    input.insert(
        0,
        serde_json::json!({
            "type": "message",
            "role": "system",
            "content": [{"type":"input_text", "text":"x".repeat(4_096)}]
        }),
    );
    input.insert(
        2,
        serde_json::json!({
            "type": "function_call",
            "name": "query",
            "arguments": "{}"
        }),
    );
    let program_sha256 = canonical_json_sha256(&program).expect("program digest");
    let programs = BTreeMap::from([(program_sha256, program.clone())]);
    let support =
        vec![receipt_with_program_atoms(&example, true, &programs).expect("support receipt")];
    let mut bucket = OnlineCollectionBucket {
        bucket_id: "e".repeat(64),
        archetype_id: "f".repeat(64),
        programs,
        common_request_atom_ids: BTreeSet::new(),
        support,
        future: Vec::new(),
        runtime_examples: BTreeMap::new(),
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };
    let full_bytes = serde_cbor::to_vec(&example.example)
        .expect("full example")
        .len();
    insert_runtime_example(&mut bucket, &example, 128);
    let compact = bucket
        .runtime_examples
        .get(&example.evidence_graph_sha256)
        .expect("compact example");
    let compact_bytes = serde_cbor::to_vec(compact).expect("compact example").len();
    assert!(compact_bytes < full_bytes / 2);
    assert_eq!(
        independently_verified_authority_response(&program, compact),
        independently_verified_authority_response(&program, &example.example)
    );
    assert_eq!(
        compact.provider_payload["input"]
            .as_array()
            .expect("compact input")
            .len(),
        2
    );
}

#[test]
fn runtime_reservoir_preserves_top_law_under_byte_pressure() {
    let seed = observation(1, "3");
    let top_program = enumerate_source_neutral_response_programs(&seed.example)
        .expect("version space")
        .programs
        .into_iter()
        .find(|program| {
            is_source_neutral_response_program(program) && program_any(program, is_count_operation)
        })
        .expect("count program");
    let mut secondary_program = top_program.clone();
    let crate::ResponseOperation::ComposeCollection { max_items, .. } =
        &mut secondary_program.operation
    else {
        panic!("count program");
    };
    *max_items = max_items.saturating_sub(1).max(1);
    let top_digest = canonical_json_sha256(&top_program).expect("top digest");
    let secondary_digest = canonical_json_sha256(&secondary_program).expect("secondary digest");
    assert_ne!(
        response_law_key(&top_program).expect("top law"),
        response_law_key(&secondary_program).expect("secondary law")
    );
    let mut support = Vec::new();
    let mut runtime_examples = BTreeMap::new();
    for index in 1..=60 {
        let mut example = observation(index, "3");
        example.example.provider_payload["retained_noise"] = Value::String("x".repeat(50_000));
        let mut receipt = receipt(&example, true).expect("receipt");
        receipt.matched_program_sha256 = vec![if index <= 40 {
            top_digest.clone()
        } else {
            secondary_digest.clone()
        }];
        support.push(receipt);
        runtime_examples.insert(
            example.evidence_graph_sha256.clone(),
            example.example.clone(),
        );
    }
    let mut bucket = OnlineCollectionBucket {
        bucket_id: "1".repeat(64),
        archetype_id: "2".repeat(64),
        programs: BTreeMap::from([
            (top_digest.clone(), top_program),
            (secondary_digest, secondary_program),
        ]),
        common_request_atom_ids: BTreeSet::new(),
        support,
        future: Vec::new(),
        runtime_examples,
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };
    trim_bucket_runtime_examples(&mut bucket, 128);
    let retained_top_law = bucket
        .support
        .iter()
        .filter(|receipt| {
            receipt.matched_program_sha256 == [top_digest.clone()]
                && bucket
                    .runtime_examples
                    .contains_key(&receipt.evidence_graph_sha256)
        })
        .count();
    assert!(retained_top_law >= 32, "retained {retained_top_law}");
    assert!(
        persisted_runtime_example_bytes(&bucket.runtime_examples)
            <= MAX_PERSISTED_PARITY_BYTES_PER_BUCKET
    );
}

#[test]
fn unguarded_consensus_unifies_equivalent_selectors_and_abstains_on_disagreement() {
    let make_observation = |index: usize, left: Value, right: Value| OnlineCollectionObservation {
        evidence_graph_sha256: format!("{:064x}", index + 70_000),
        client_intent_id_sha256: format!("{:064x}", index + 80_000),
        session_id_sha256: format!("{:064x}", index % 4 + 90_000),
        event_time_unix_nanos: Some(index as u64),
        estimated_input_tokens: 100,
        example: CollectionSynthesisExample {
            provider_payload: json!({
                "input": [
                    {"type":"message","role":"user","content":[{
                        "type":"input_text","text":"Return the selected value"
                    }]},
                    {"type":"function_call_output","output":json!({"left":left}).to_string()},
                    {"type":"function_call_output","output":json!({"right":right}).to_string()}
                ]
            }),
            expected_response: "3".to_owned(),
        },
    };
    let left = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::TurnOutputScalarOrdinal {
            output_ordinal: 1,
            scalar_ordinal: 0,
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    let right = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::TurnOutputScalarOrdinal {
            output_ordinal: 2,
            scalar_ordinal: 0,
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    let left_digest = canonical_json_sha256(&left).expect("left digest");
    let right_digest = canonical_json_sha256(&right).expect("right digest");
    let programs = BTreeMap::from([(left_digest.clone(), left), (right_digest.clone(), right)]);
    let observations = (1_usize..=4)
        .map(|index| {
            if index.is_multiple_of(2) {
                make_observation(index, json!(3), json!("n/a"))
            } else {
                make_observation(index, json!("n/a"), json!(3))
            }
        })
        .collect::<Vec<_>>();
    let support = observations
        .iter()
        .enumerate()
        .map(|(index, observation)| {
            let mut value =
                receipt_with_program_atoms(observation, true, &programs).expect("support receipt");
            value.matched_program_sha256 = vec![if index.is_multiple_of(2) {
                left_digest.clone()
            } else {
                right_digest.clone()
            }];
            value
        })
        .collect::<Vec<_>>();
    let runtime_examples = observations
        .iter()
        .map(|observation| {
            (
                observation.evidence_graph_sha256.clone(),
                observation.example.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for observation in &observations {
        let responses = programs
            .values()
            .filter_map(|program| {
                independently_verified_authority_response(program, &observation.example)
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(responses, BTreeSet::from(["3".to_owned()]));
    }
    let manual_consensus = ResponseProgram::unique_consensus(
        programs
            .values()
            .cloned()
            .map(|program| ResponseConsensusVariant {
                program,
                allowed_layout_sha256: Vec::new(),
                required_request_atom_ids: Vec::new(),
            })
            .collect(),
    );
    for observation in &observations {
        assert_eq!(
            independently_verified_authority_response_result(
                &manual_consensus,
                &observation.example
            ),
            Ok("3".to_owned())
        );
    }
    let bucket = OnlineCollectionBucket {
        bucket_id: "a".repeat(64),
        archetype_id: "project".to_owned(),
        programs,
        common_request_atom_ids: BTreeSet::new(),
        support,
        future: Vec::new(),
        runtime_examples,
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };
    let candidate = match support_consensus_candidate(&bucket).expect("candidate") {
        SupportConsensusCandidate::Ready(candidate) => candidate,
        SupportConsensusCandidate::Blocked(reason) => panic!("blocked: {reason}"),
    };
    assert!(matches!(
        &candidate.operation,
        crate::ResponseOperation::UniqueConsensus { .. }
    ));
    for observation in &observations {
        assert_eq!(
            independently_verified_authority_response(&candidate, &observation.example).as_deref(),
            Some("3")
        );
    }

    let conflict = make_observation(1, json!(3), json!(4));
    let execution = execute_response(&candidate, "", &conflict.example.provider_payload);
    assert_eq!(execution.status, ResponseExecutionStatus::Abstain);
    let mut conflicting_bucket = bucket;
    conflicting_bucket.runtime_examples.insert(
        conflict.evidence_graph_sha256.clone(),
        conflict.example.clone(),
    );
    assert!(!matches!(
        support_consensus_candidate(&conflicting_bucket).expect("conflicting candidate"),
        SupportConsensusCandidate::Ready(_)
    ));
}

#[test]
fn keyed_surface_layout_routes_actor_and_independent_verifier() {
    let alpha_payload = serde_json::json!({
        "input": [{"type":"function_call_output", "output":"{\"alpha\":7}"}]
    });
    let beta_payload = serde_json::json!({
        "input": [{"type":"function_call_output", "output":"{\"beta\":8}"}]
    });
    let alpha_layout = structural_layout_sha256(&alpha_payload).expect("alpha layout");
    let beta_layout = structural_layout_sha256(&beta_payload).expect("beta layout");
    assert_ne!(alpha_layout, beta_layout);
    let consensus = ResponseProgram::unique_consensus(vec![
        ResponseConsensusVariant {
            program: ResponseProgram::project_selected_value(
                crate::ResponseValueSelector::JsonField {
                    field: "alpha".to_owned(),
                    value_type: crate::AtomValueType::Integer,
                },
                crate::ValueProjectionFormat::PlainText,
                "completed",
            ),
            allowed_layout_sha256: vec![alpha_layout],
            required_request_atom_ids: Vec::new(),
        },
        ResponseConsensusVariant {
            program: ResponseProgram::project_selected_value(
                crate::ResponseValueSelector::JsonField {
                    field: "beta".to_owned(),
                    value_type: crate::AtomValueType::Integer,
                },
                crate::ValueProjectionFormat::PlainText,
                "completed",
            ),
            allowed_layout_sha256: vec![beta_layout],
            required_request_atom_ids: Vec::new(),
        },
    ]);
    consensus.validate().expect("consensus");
    for (provider_payload, expected_response) in [(alpha_payload, "7"), (beta_payload, "8")] {
        let example = CollectionSynthesisExample {
            provider_payload,
            expected_response: expected_response.to_owned(),
        };
        assert_eq!(
            independently_verified_authority_response(&consensus, &example).as_deref(),
            Some(expected_response)
        );
    }
}

#[test]
fn request_phase_guard_routes_same_layout_adapters() {
    let payload = |request: &str| {
        serde_json::json!({
            "input": [
                {"type":"message", "role":"user", "content":[{"type":"input_text", "text":request}]},
                {"type":"function_call_output", "output":"{\"alpha\":7,\"beta\":8}"}
            ]
        })
    };
    let alpha_payload = payload("select alpha");
    let beta_payload = payload("select beta");
    let layout = structural_layout_sha256(&alpha_payload).expect("layout");
    assert_eq!(
        layout,
        structural_layout_sha256(&beta_payload).expect("same layout")
    );
    let alpha_atoms = request_phase_atom_ids("select alpha");
    let beta_atoms = request_phase_atom_ids("select beta");
    let alpha_guard = alpha_atoms
        .iter()
        .copied()
        .find(|atom| beta_atoms.binary_search(atom).is_err())
        .expect("alpha discriminator");
    let beta_guard = beta_atoms
        .iter()
        .copied()
        .find(|atom| alpha_atoms.binary_search(atom).is_err())
        .expect("beta discriminator");
    let consensus = ResponseProgram::unique_consensus(vec![
        ResponseConsensusVariant {
            program: ResponseProgram::project_selected_value(
                crate::ResponseValueSelector::JsonField {
                    field: "alpha".to_owned(),
                    value_type: crate::AtomValueType::Integer,
                },
                crate::ValueProjectionFormat::PlainText,
                "completed",
            ),
            allowed_layout_sha256: vec![layout.clone()],
            required_request_atom_ids: vec![alpha_guard],
        },
        ResponseConsensusVariant {
            program: ResponseProgram::project_selected_value(
                crate::ResponseValueSelector::JsonField {
                    field: "beta".to_owned(),
                    value_type: crate::AtomValueType::Integer,
                },
                crate::ValueProjectionFormat::PlainText,
                "completed",
            ),
            allowed_layout_sha256: vec![layout],
            required_request_atom_ids: vec![beta_guard],
        },
    ]);
    consensus.validate().expect("consensus");
    for (provider_payload, expected_response) in [(alpha_payload, "7"), (beta_payload, "8")] {
        let example = CollectionSynthesisExample {
            provider_payload,
            expected_response: expected_response.to_owned(),
        };
        assert_eq!(
            independently_verified_authority_response(&consensus, &example).as_deref(),
            Some(expected_response)
        );
    }
}

#[test]
fn request_referenced_json_field_has_actor_verifier_parity() {
    let payload = |request: &str| {
        json!({
            "input": [
                {"type":"message", "role":"user", "content":[{
                    "type":"input_text", "text":request
                }]},
                {"type":"function_call_output", "output":"{\"alpha\":7,\"beta\":8}"}
            ]
        })
    };
    let program = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::RequestReferencedJsonField {
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    program.validate().expect("program");
    let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
    let selected = payload("Return alpha");
    let execution = execute_response(&program, "", &selected);
    assert_eq!(execution.response.as_deref(), Some("7"));
    assert!(verify_response_independently(&verifier, &selected, "7").is_ok());

    let ambiguous = payload("Return alpha and beta");
    assert_eq!(
        execute_response(&program, "", &ambiguous).status,
        ResponseExecutionStatus::Abstain
    );
    assert!(verify_response_independently(&verifier, &ambiguous, "7").is_err());

    let from_end = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal: 0,
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    let from_end_verifier =
        source_neutral_verifier_for_program(&from_end).expect("from-end verifier");
    let from_end_payload = json!({
        "input": [
            {"type":"message", "role":"user", "content":"Return the result"},
            {"type":"function_call_output", "output":"{\"values\":[1,2,7]}"}
        ]
    });
    assert_eq!(
        execute_response(&from_end, "", &from_end_payload)
            .response
            .as_deref(),
        Some("7")
    );
    assert!(verify_response_independently(&from_end_verifier, &from_end_payload, "7").is_ok());
    assert!(verify_response_independently(&from_end_verifier, &from_end_payload, "2").is_err());
}

#[test]
fn phase_ranked_adapter_selects_unique_physical_role_and_abstains_on_tie() {
    let program = |ordinal: u16| {
        ResponseProgram::project_selected_value(
            crate::ResponseValueSelector::JsonScalarOrdinal {
                ordinal,
                value_type: crate::AtomValueType::Integer,
            },
            crate::ValueProjectionFormat::PlainText,
            "completed",
        )
    };
    let alpha = program(0);
    let beta = program(1);
    let alpha_digest = canonical_json_sha256(&alpha).expect("alpha digest");
    let beta_digest = canonical_json_sha256(&beta).expect("beta digest");
    let programs = BTreeMap::from([(alpha_digest, alpha), (beta_digest, beta)]);
    let observation = |index: usize, requested: &str, alpha: i64, beta: i64| {
        let expected = if requested == "alpha" { alpha } else { beta };
        OnlineCollectionObservation {
            evidence_graph_sha256: format!("{index:064x}"),
            client_intent_id_sha256: format!("{:064x}", index + 100),
            session_id_sha256: format!("{:064x}", index % 2 + 200),
            event_time_unix_nanos: Some(index as u64),
            estimated_input_tokens: 100,
            example: CollectionSynthesisExample {
                provider_payload: serde_json::json!({
                    "input": [
                        {"type":"message", "role":"user", "content":[{
                            "type":"input_text",
                            "text":format!("Return {requested}")
                        }]},
                        {"type":"function_call_output", "output":serde_json::json!({
                            "alpha":alpha,
                            "beta":beta
                        }).to_string()}
                    ]
                }),
                expected_response: expected.to_string(),
            },
        }
    };
    let support_observations = [
        observation(1, "alpha", 11, 21),
        observation(2, "beta", 12, 22),
        observation(3, "alpha", 13, 23),
        observation(4, "beta", 14, 24),
    ];
    let support = support_observations
        .iter()
        .map(|observation| {
            receipt_with_program_atoms(observation, true, &programs).expect("receipt")
        })
        .collect::<Vec<_>>();
    let runtime_examples = support_observations
        .iter()
        .map(|observation| {
            (
                observation.evidence_graph_sha256.clone(),
                observation.example.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut bucket = OnlineCollectionBucket {
        bucket_id: "a".repeat(64),
        archetype_id: "project".to_owned(),
        programs,
        common_request_atom_ids: BTreeSet::new(),
        support,
        future: Vec::new(),
        runtime_examples,
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    };
    refresh_durable_adapter_phase_atoms(&mut bucket);
    assert_eq!(bucket.durable_adapter_phase_atoms.len(), 4);
    bucket.runtime_examples.clear();
    let candidate = phase_ranked_semantic_adapters(&bucket).expect("wave candidate");
    for future in [
        observation(5, "alpha", 105, 205),
        observation(6, "beta", 106, 206),
    ] {
        assert_eq!(
            independently_verified_authority_response(&candidate, &future.example),
            Some(future.example.expected_response)
        );
    }
    let mut prose = observation(8, "alpha", 108, 208);
    prose.example.expected_response = "Selected: 108.".to_owned();
    assert!(response_program_authority_matches_example(
        &candidate,
        &prose.example
    ));
    assert_eq!(
        independently_verified_authority_response(&candidate, &prose.example).as_deref(),
        Some("108")
    );
    let ambiguous = observation(7, "result", 107, 207);
    let execution = execute_response(&candidate, "", &ambiguous.example.provider_payload);
    assert_eq!(execution.status, ResponseExecutionStatus::Abstain);
    assert!(independently_verified_authority_response(&candidate, &ambiguous.example).is_none());

    let root = std::env::temp_dir().join(format!(
        "nando-adapter-wave-maintenance-{}-{}",
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
    let bucket_id = bucket.bucket_id.clone();
    miner.checkpoint.buckets = vec![bucket];
    miner
        .checkpoint
        .structural_resynthesis_pending_bucket_ids
        .insert(bucket_id);
    miner
        .run_structural_resynthesis_work_slice()
        .expect("proof refresh");
    let frozen = miner
        .checkpoint
        .buckets
        .iter()
        .find(|bucket| bucket.frozen_program_sha256.is_some())
        .expect("frozen law subcenter");
    assert_eq!(frozen.future.len(), 0);
    assert_eq!(frozen.wrong_accepts, 0);
    fs::remove_dir_all(root).expect("cleanup");
}

fn program_any(
    program: &ResponseProgram,
    predicate: fn(&crate::ResponseOperation) -> bool,
) -> bool {
    predicate(&program.operation)
        || matches!(
            &program.operation,
            crate::ResponseOperation::UniqueConsensus { variants, .. }
                if variants.iter().any(|variant| program_any(&variant.program, predicate))
        )
}

fn is_count_operation(operation: &crate::ResponseOperation) -> bool {
    matches!(
        operation,
        crate::ResponseOperation::ComposeCollection { steps, .. }
            if steps.iter().any(|step| matches!(step, crate::CollectionProgramStep::Count))
    )
}

fn is_multi_output_project(operation: &crate::ResponseOperation) -> bool {
    matches!(
        operation,
        crate::ResponseOperation::ProjectSelectedValue {
            selector: crate::ResponseValueSelector::UniqueTurnScalar { .. },
            renderer: crate::CollectionOutputRenderer::RenderSequence { .. },
            ..
        }
    )
}

fn observation(index: usize, expected: &str) -> OnlineCollectionObservation {
    let field = format!("surface_{index}");
    let base = i64::try_from(index).unwrap_or_default().saturating_mul(10);
    OnlineCollectionObservation {
        evidence_graph_sha256: format!("{index:064x}"),
        client_intent_id_sha256: format!("{:064x}", index + 10_000),
        session_id_sha256: format!("{:064x}", index % 4 + 20_000),
        event_time_unix_nanos: Some(index as u64),
        estimated_input_tokens: 100,
        example: CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{
                        "type":"input_text",
                        "text":format!("Count records for batch {index}")
                    }]},
                    {"type":"function_call_output","output":json!({
                        field: [
                            {"value":base + 1},
                            {"value":base + 2},
                            {"value":base + 3}
                        ]
                    }).to_string()}
                ]
            }),
            expected_response: expected.to_owned(),
        },
    }
}

fn ordinal_count_observation(
    index: usize,
    expected: &str,
    output_ordinal: u16,
) -> OnlineCollectionObservation {
    let mut observation = observation(index, expected);
    let input = observation.example.provider_payload["input"]
        .as_array_mut()
        .expect("input array");
    input[0]["content"][0]["text"] = Value::String("Count the records".to_owned());
    let noise = json!({
        "type":"function_call_output",
        "output":"command completed"
    });
    match output_ordinal {
        1 => input.push(noise),
        2 => input.insert(1, noise),
        _ => panic!("unsupported test ordinal"),
    }
    observation
}

fn routed_observation(
    index: usize,
    expected: &str,
    prompt: &str,
    alternate_layout: bool,
) -> OnlineCollectionObservation {
    let mut observation = observation(index, expected);
    observation.example.provider_payload["input"][0]["content"][0]["text"] =
        Value::String(prompt.to_owned());
    if alternate_layout {
        let output = observation.example.provider_payload["input"][1]["output"]
            .as_str()
            .expect("tool output");
        let mut parsed = serde_json::from_str::<Value>(output).expect("tool json");
        parsed["layout_marker"] = Value::Bool(true);
        observation.example.provider_payload["input"][1]["output"] =
            Value::String(parsed.to_string());
    }
    observation
}

fn multi_output_observation(index: usize) -> OnlineCollectionObservation {
    let total_field = format!("total_surface_{index}");
    let status_field = format!("status_surface_{index}");
    let total = index.saturating_add(40);
    let status = format!("ready-{index}");
    let total_output = if index.is_multiple_of(2) {
        json!({"wrapper": {(total_field.clone()): total}})
    } else {
        json!({(total_field): total})
    };
    let status_output = if index.is_multiple_of(2) {
        json!({"result": {(status_field.clone()): status}})
    } else {
        json!({(status_field): status})
    };
    OnlineCollectionObservation {
        evidence_graph_sha256: format!("{:064x}", index + 30_000),
        client_intent_id_sha256: format!("{:064x}", index + 40_000),
        session_id_sha256: format!("{:064x}", index % 4 + 50_000),
        event_time_unix_nanos: Some(index as u64),
        estimated_input_tokens: 1_000,
        example: CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{
                        "type":"input_text","text":"Summarize the verified result"
                    }]},
                    {"type":"function_call_output","call_id":format!("a-{index}"),"output":total_output.to_string()},
                    {"type":"function_call_output","call_id":format!("b-{index}"),"output":status_output.to_string()}
                ]
            }),
            expected_response: format!("Total: {total}; status: ready-{index}."),
        },
    }
}

#[test]
fn different_teacher_surfaces_converge_to_one_canonical_program() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-surface-convergence-{}-{}",
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
    for (index, surface) in [
        "Total records: 3",
        "Count: 3",
        "Found 3 records",
        "Matching rows: 3",
        "There are 3 rows",
        "Verified count is 3",
        "The batch contains 3 records",
        "Result: 3 items",
    ]
    .into_iter()
    .enumerate()
    {
        let observation = observation(index + 1, surface);
        if index < 4 {
            miner
                .observe_replay_training_buffered(observation)
                .expect("support");
        } else {
            miner.observe(observation).expect("future");
        }
    }

    let status = miner.status();
    assert_eq!(
        status
            .exact_executable_observations_total
            .saturating_add(status.semantic_executable_observations_total),
        8
    );
    assert_eq!(status.unsupported_total, 0);
    assert_eq!(status.frozen_buckets_total, 1);
    assert_eq!(status.future_receipts_unique_total, 4);
    assert_eq!(status.wrong_accepts_total, 0);
    assert!(miner.checkpoint.buckets.len() <= 2);
    assert!(
        miner
            .checkpoint
            .buckets
            .iter()
            .flat_map(|bucket| bucket.programs.values())
            .any(|program| program_any(program, is_count_operation))
    );
    assert_eq!(miner.quarantine_packages().expect("packages").len(), 1);
    fs::remove_dir_all(root).ok();
}

#[test]
fn version_space_restart_preserves_privacy_safe_runtime_parity_receipts() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.json");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 4,
        future_rows: 4,
        max_buckets: 8,
        max_receipts_per_bucket: 16,
    };
    let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
    for index in 1..=4 {
        miner.observe(observation(index, "3")).expect("observe");
    }
    let support_only_package_id = miner
        .quarantine_packages()
        .expect("support package")
        .into_iter()
        .next()
        .expect("frozen package")
        .package_id;
    let mut late = observation(9, "3");
    late.event_time_unix_nanos = Some(3);
    miner.observe(late).expect("late after freeze");
    let mut leaked_intent = observation(10, "3");
    leaked_intent.client_intent_id_sha256 = observation(1, "3").client_intent_id_sha256;
    miner
        .observe(leaked_intent)
        .expect("support intent after freeze");
    for index in 5..=8 {
        miner.observe(observation(index, "3")).expect("future");
    }
    miner
        .observe(observation(8, "3"))
        .expect("duplicate observation");
    let status = miner.status();
    assert_eq!(status.observations_total, 10);
    assert_eq!(status.duplicate_observations_total, 1);
    assert_eq!(status.late_after_freeze_total, 1);
    assert_eq!(status.future_intent_rejected_total, 1);
    assert_eq!(status.full_enumerations_total, 1);
    assert_eq!(status.version_space_intersection_checks_total, 3);
    assert_eq!(status.guard_scheduled_buckets_total, 3);
    assert_eq!(status.guard_pruned_buckets_total, 0);
    assert_eq!(status.buckets.len(), 1);
    assert!(status.buckets[0].frozen);
    assert_eq!(status.buckets[0].support_rows, 4);
    assert_eq!(status.buckets[0].future_rows, 4);
    assert_eq!(status.buckets[0].wrong_accepts, 0);
    assert!(status.buckets[0].support_manifest_sha256.is_some());
    assert!(status.buckets[0].future_manifest_sha256.is_some());
    let packages = miner.quarantine_packages().expect("packages");
    assert_eq!(packages.len(), 1);
    assert_ne!(packages[0].package_id, support_only_package_id);
    assert_eq!(packages[0].state, ResponsePackageState::Quarantine);
    assert!(!packages[0].eligible_for_admission_candidate());
    drop(miner);
    let durable = fs::read(&path).expect("checkpoint");
    assert!(durable.starts_with(ONLINE_COLLECTION_CHECKPOINT_MAGIC_V3));
    assert!(
        !durable
            .windows(b"surface_".len())
            .any(|row| row == b"surface_")
    );
    assert!(
        !durable
            .windows(b"provider_payload".len())
            .any(|row| row == b"provider_payload")
    );
    let restored = OnlineCollectionMiner::open(&path, config).expect("restart");
    assert_eq!(restored.status(), status);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn v6_rebuilds_exact_renderer_candidates_without_claiming_evidence() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-v6-renderer-{}-{}",
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
    let evidence_id = "f".repeat(64);
    let example = CollectionSynthesisExample {
        provider_payload: json!({
            "input":[{"type":"function_call_output","output":"{\"ok\":3}"}]
        }),
        expected_response: "Успешных записей: 3".to_owned(),
    };
    let teacher = ResponseProgram::project_selected_value(
        crate::ResponseValueSelector::UniqueTurnScalar {
            value_type: crate::AtomValueType::Integer,
        },
        crate::ValueProjectionFormat::PlainText,
        "completed",
    );
    let teacher_digest = canonical_json_sha256(&teacher).expect("teacher digest");
    let archetype_id = response_program_archetype_id(&teacher).expect("archetype");
    miner.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V5;
    miner.checkpoint.observations_total = 1;
    miner.checkpoint.unsupported_total = 1;
    miner.checkpoint.teacher_only_observations_total = 1;
    miner
        .checkpoint
        .observed_evidence_graph_sha256
        .insert(evidence_id.clone());
    miner.checkpoint.buckets.push(OnlineCollectionBucket {
        bucket_id: "legacy-teacher".to_owned(),
        archetype_id,
        programs: BTreeMap::from([(teacher_digest, teacher)]),
        common_request_atom_ids: BTreeSet::new(),
        support: Vec::new(),
        future: Vec::new(),
        runtime_examples: BTreeMap::from([(evidence_id, example.clone())]),
        durable_adapter_phase_atoms: BTreeMap::new(),
        durable_runtime_parity_receipts: BTreeMap::new(),
        adaptive_candidate_freeze: None,
        frozen_program_sha256: None,
        support_watermark_event_time_unix_nanos: None,
        support_manifest_sha256: None,
        rejected_program_sha256: BTreeSet::new(),
        learned_anti_atom_ids: BTreeSet::new(),
        wrong_accepts: 0,
    });
    miner.persist().expect("persist v5 checkpoint");
    drop(miner);

    let migrated = OnlineCollectionMiner::open(&path, config).expect("migrate v9");
    let status = migrated.status();
    assert_eq!(
        status.pooling_strategy_version,
        ONLINE_COLLECTION_POOLING_STRATEGY_V36
    );
    assert_eq!(status.renderer_consensus_migrated_examples_total, 1);
    assert_eq!(status.support_receipts_unique_total, 0);
    assert_eq!(status.future_receipts_unique_total, 0);
    assert!(migrated.checkpoint.buckets.iter().any(|bucket| {
        bucket.programs.values().any(|program| {
            crate::response_program_exactly_matches_example(program, &example)
                && !is_source_neutral_response_program(program)
        })
    }));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn v21_restart_resynthesizes_retained_support_without_creating_future() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-v21-resynthesis-{}-{}",
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
        max_buckets: 32,
        max_receipts_per_bucket: 16,
    };
    let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
    for index in 1..=4 {
        miner
            .observe(multi_output_observation(index))
            .expect("support");
    }
    let bucket_index = miner
        .checkpoint
        .buckets
        .iter()
        .position(|bucket| {
            bucket.support.len() >= config.support_rows
                && bucket
                    .programs
                    .values()
                    .any(|program| canonical_dynamic_role_count(program) >= 2)
        })
        .expect("multi-scalar support bucket");
    let removed = miner.checkpoint.buckets[bucket_index]
        .programs
        .iter()
        .filter_map(|(digest, program)| {
            let example = miner.checkpoint.buckets[bucket_index]
                .runtime_examples
                .values()
                .next()?;
            let response = independently_verified_authority_response(program, example)?;
            (canonical_dynamic_role_count(program) >= 2 && response != example.expected_response)
                .then_some(digest.clone())
        })
        .collect::<BTreeSet<_>>();
    assert!(
        !removed.is_empty(),
        "canonical law must exist before downgrade"
    );
    let bucket = &mut miner.checkpoint.buckets[bucket_index];
    bucket
        .programs
        .retain(|digest, _| !removed.contains(digest));
    assert!(
        !bucket.programs.is_empty(),
        "surface programs remain in V20"
    );
    for receipt in &mut bucket.support {
        receipt
            .matched_program_sha256
            .retain(|digest| !removed.contains(digest));
    }
    bucket.frozen_program_sha256 = None;
    bucket.support_watermark_event_time_unix_nanos = None;
    bucket.support_manifest_sha256 = None;
    bucket.future.clear();
    bucket.durable_runtime_parity_receipts.clear();
    miner.checkpoint.pooling_strategy_version = ONLINE_COLLECTION_POOLING_STRATEGY_V20;
    miner.persist().expect("persist V20 checkpoint");
    drop(miner);

    let mut restored = OnlineCollectionMiner::open(&path, config).expect("migrate to V21");
    assert!(restored.has_structural_resynthesis_work());
    while restored.has_structural_resynthesis_work() {
        restored
            .run_structural_resynthesis_work_slice()
            .expect("bounded structural resynthesis");
    }
    let status = restored.status();
    assert_eq!(
        status.pooling_strategy_version,
        ONLINE_COLLECTION_POOLING_STRATEGY_V36
    );
    assert_eq!(status.future_receipts_unique_total, 0);
    assert_eq!(status.wrong_accepts_total, 0);
    assert_eq!(status.structural_resynthesis_pending_buckets, 0);
    assert!(status.structural_resynthesis_completed_buckets_total >= 1);
    assert_eq!(status.structural_resynthesis_failed_buckets_total, 0);
    assert!(restored.checkpoint.buckets.iter().any(|bucket| {
        bucket.programs.values().any(|program| {
            canonical_dynamic_role_count(program) >= 2
                && bucket.runtime_examples.values().any(|example| {
                    independently_verified_authority_response(program, example)
                        .is_some_and(|response| response != example.expected_response)
                })
        })
    }));
    assert!(status.support_receipts_unique_total >= config.support_rows);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn semantic_program_pool_survives_field_renames_and_collects_future() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-outcome-multi-source-{}-{}",
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
        max_buckets: 32,
        max_receipts_per_bucket: 16,
    };
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.cbor"), config).expect("miner");
    for index in 1..=4 {
        miner
            .observe(multi_output_observation(index))
            .expect("support");
    }
    for index in 5..=8 {
        miner
            .observe(multi_output_observation(index))
            .expect("future");
    }
    let package = miner
        .quarantine_packages()
        .expect("packages")
        .into_iter()
        .find(|package| program_any(&package.program, is_multi_output_project))
        .expect("portable multi-output package");
    assert_eq!(package.proof.support_rows, 4);
    assert_eq!(package.proof.future_rows, 4);
    assert_eq!(package.proof.wrong_accepts, 0);
    assert!(package.proof.distinct_surfaces >= 2);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn semantic_count_inside_teacher_prose_reaches_external_admission() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-semantic-count-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let checkpoint_path = root.join("checkpoint.cbor");
    let config = legacy_default_config();
    let mut miner = OnlineCollectionMiner::open(&checkpoint_path, config).expect("miner");
    for index in 1..=32 {
        miner
            .observe(routed_observation(
                index,
                "Total records: 3",
                "Count records for this batch",
                index.is_multiple_of(2),
            ))
            .expect("semantic count observation");
    }
    miner
        .observe(routed_observation(
            10_000,
            "Rows: 3",
            "Summarize the selected payload",
            false,
        ))
        .expect("competing deterministic family");
    for index in 33..=64 {
        miner
            .observe(routed_observation(
                index,
                "Total records: 3",
                "Count records for this batch",
                index.is_multiple_of(2),
            ))
            .expect("semantic count future");
    }
    let package = miner
        .quarantine_packages()
        .expect("packages")
        .into_iter()
        .find(|package| program_any(&package.program, is_count_operation))
        .expect("semantic count package");
    assert_eq!(package.proof.support_rows, 32);
    assert_eq!(package.proof.future_rows, 32);
    assert_eq!(package.proof.wrong_accepts, 0);
    let heldout = routed_observation(
        100,
        "Total records: 3",
        "Count records for this batch",
        true,
    );
    let execution = execute_response(&package.program, "", &heldout.example.provider_payload);
    assert_eq!(execution.response.as_deref(), Some("3"));
    assert!(
        verify_response_independently(
            package.verifier.as_ref().expect("verifier"),
            &heldout.example.provider_payload,
            "3"
        )
        .is_ok()
    );
    drop(miner);
    let miner = OnlineCollectionMiner::open(&checkpoint_path, config).expect("restart miner");
    let candidates = miner.admission_candidates().expect("candidates");
    let candidate = candidates
        .into_iter()
        .find(|candidate| program_any(&candidate.package.program, is_count_operation))
        .unwrap_or_else(|| {
            let diagnostics = miner
                .checkpoint
                .buckets
                .iter()
                .enumerate()
                .filter_map(|(index, bucket)| {
                    let mut package = miner.package_for_bucket(index, bucket, false).ok()??;
                    let causal = miner.collection_causal_report(bucket, &package).ok()?;
                    package.state = ResponsePackageState::Active;
                    package.proof.wave_causal_pass = causal.verdict == "PASS";
                    package.wave_margin_micro = causal.wave_margin_micro;
                    Some((causal, package.admission_candidate_blocker()))
                })
                .collect::<Vec<_>>();
            panic!(
                "semantic count admission candidate: {:#?}\ndiagnostics={diagnostics:#?}",
                miner.status()
            )
        });
    assert_eq!(candidate.future_receipts.len(), 32);
    assert_eq!(candidate.runtime_parity_cases.len(), 0);
    assert_eq!(candidate.durable_runtime_parity_receipts.len(), 32);
    let mut tampered = candidate.clone();
    tampered.durable_runtime_parity_receipts[0].input_sha256 = "f".repeat(64);
    assert!(
        crate::build_online_collection_admission_snapshot(
            &[tampered],
            "project",
            1,
            100,
            60,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("tampered admission")
        .is_none()
    );
    let snapshot = crate::build_online_collection_admission_snapshot(
        &[candidate],
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
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn multi_output_semantic_program_reaches_external_admission() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-outcome-admission-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let mut miner =
        OnlineCollectionMiner::open(root.join("checkpoint.cbor"), legacy_default_config())
            .expect("miner");
    for index in 1..=32 {
        miner
            .observe(multi_output_observation(index))
            .expect("support");
    }
    let mut competing = multi_output_observation(10_000);
    competing.example.provider_payload["input"][0]["content"][0]["text"] =
        Value::String("Emit an alternate verified template".to_owned());
    competing.example.expected_response = "Result: 10040; status: ready-10000.".to_owned();
    miner.observe(competing).expect("competing family");
    for index in 33..=64 {
        miner
            .observe(multi_output_observation(index))
            .expect("future");
    }
    let status = miner.status();
    assert_eq!(
        status.pooling_strategy_version,
        ONLINE_COLLECTION_POOLING_STRATEGY_V36
    );
    assert!(
        status
            .exact_executable_observations_total
            .saturating_add(status.semantic_executable_observations_total)
            >= 32,
        "{status:#?}"
    );
    assert!(status.program_pool_receipts_total >= 32);
    assert!(status.frozen_buckets_total >= 1, "{status:#?}");
    assert!(status.pre_admission_ready_buckets_total >= 1, "{status:#?}");
    assert!(status.support_receipts_unique_total >= 32);
    assert!(status.future_receipts_unique_total >= 32);
    assert_eq!(status.wrong_accepts_total, 0);
    assert!(status.runtime_parity_cases_total >= 32);
    assert!(
        status
            .frozen_program_kinds
            .get("project")
            .copied()
            .unwrap_or(0)
            >= 1
    );
    let candidates = miner.admission_candidates().expect("candidates");
    let causal_reports = miner
        .checkpoint
        .buckets
        .iter()
        .enumerate()
        .filter_map(|(index, bucket)| {
            miner
                .package_for_bucket(index, bucket, false)
                .ok()
                .flatten()
                .and_then(|package| miner.collection_causal_report(bucket, &package).ok())
        })
        .collect::<Vec<_>>();
    let candidate_blockers = miner
        .checkpoint
        .buckets
        .iter()
        .enumerate()
        .filter_map(|(index, bucket)| {
            let mut package = miner.package_for_bucket(index, bucket, false).ok()??;
            let causal = miner.collection_causal_report(bucket, &package).ok()?;
            package.state = ResponsePackageState::Active;
            package.proof.wave_causal_pass = causal.verdict == "PASS";
            package.wave_margin_micro = causal.wave_margin_micro;
            Some(package.admission_candidate_blocker())
        })
        .collect::<Vec<_>>();
    let candidate = candidates
        .iter()
        .find(|candidate| program_any(&candidate.package.program, is_multi_output_project))
        .unwrap_or_else(|| {
            panic!(
                "admission-ready multi-output candidate: {:#?}\ncausal={causal_reports:#?}\nblockers={candidate_blockers:#?}",
                miner.status()
            )
        });
    assert_eq!(candidate.causal_report.verdict, "PASS");
    assert_eq!(candidate.future_receipts.len(), 32);
    assert_eq!(candidate.runtime_parity_cases.len(), 0);
    assert_eq!(candidate.durable_runtime_parity_receipts.len(), 32);
    let snapshot = crate::build_online_collection_admission_snapshot(
        std::slice::from_ref(candidate),
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
fn counterexample_learns_anti_center_then_revokes_only_when_unseparable() {
    let root = std::env::temp_dir().join(format!(
        "nando-online-collection-counterexample-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("root");
    let path = root.join("checkpoint.json");
    let config = OnlineCollectionConfig {
        proof_mode: OnlineCollectionProofMode::LegacyFixedRows,
        support_rows: 4,
        future_rows: 4,
        max_buckets: 8,
        max_receipts_per_bucket: 16,
    };
    let mut miner = OnlineCollectionMiner::open(&path, config).expect("miner");
    for index in 1..=4 {
        miner.observe(observation(index, "3")).expect("support");
    }
    miner
        .observe(observation(5, "not-three"))
        .expect("counterexample");
    let status = miner.status();
    assert_eq!(status.counterexamples_total, 1);
    assert_eq!(status.revoked_candidates_total, 0);
    assert_eq!(status.buckets.len(), 1);
    assert!(status.buckets[0].frozen);
    assert_eq!(status.buckets[0].wrong_accepts, 0);
    assert!(status.buckets[0].learned_anti_atoms > 0);

    let mut unseparable = observation(6, "not-three");
    unseparable.example.provider_payload["input"][0]["content"][0]["text"] =
        Value::String("Count records for batch 1".to_owned());
    miner.observe(unseparable).expect("unseparable");
    let status = miner.status();
    assert_eq!(status.revoked_candidates_total, 1);
    assert!(!status.buckets[0].frozen);
    assert_eq!(status.buckets[0].wrong_accepts, 1);
    assert_eq!(status.buckets[0].rejected_programs, 1);
    assert!(miner.quarantine_packages().expect("packages").is_empty());
    drop(miner);
    OnlineCollectionMiner::open(&path, config).expect("restart");
    fs::remove_dir_all(root).expect("cleanup");
}

#[path = "online_collection_tests/scored.rs"]
mod scored;

#[path = "online_collection_tests/adaptive.rs"]
mod adaptive;
