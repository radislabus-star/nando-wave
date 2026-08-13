use std::collections::BTreeSet;

use super::*;

#[test]
fn queue_can_rank_immature_novel_cohort_first_but_freezes_first_ready_cohort() {
    let mut rows = ready_rows();
    rows.push(evidence_row(
        15,
        101,
        201,
        K1ConsequenceTypeV1::Collection,
        K1NaturalEvidenceClassV1::NaturalLive,
        302,
        false,
        false,
    ));
    let catalog = catalog(&rows);
    let deficit = deficit(vec![K1ConsequenceTypeV1::Scalar]);
    let queue =
        build_k1_natural_candidate_queue_v1(&catalog, &deficit, catalog_watermark(&catalog))
            .expect("queue");

    assert_eq!(queue.rows[0].score.total_k1_gain, 3);
    assert_eq!(queue.rows[0].score.readiness_rank, 0);
    let ready = queue.first_readiness_pass().expect("ready candidate");
    assert_eq!(ready.score.total_k1_gain, 2);

    let immature = catalog
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_root_sha256 == queue.rows[0].candidate_root_sha256)
        .expect("immature candidate");
    assert_eq!(
        K1NaturalCandidateFreezeV1::seal(
            1,
            &catalog,
            &deficit,
            &queue,
            immature,
            queue.rows[0].score.clone(),
            "nando.k1-operator-blind-scheduler.v1".to_owned(),
            root(706),
            K1GenerationBudgetV1 {
                maximum_support_rows: 64,
                maximum_probe_rounds: 4,
                maximum_probe_cost_units: 100,
                maximum_generation_seconds: 3_600,
            },
            immature.last_capture_sequence,
            immature.last_capture_sequence,
            1_700_000_000,
        ),
        Err("k1_candidate_freeze_binding_invalid")
    );
}

#[test]
fn equally_novel_ready_cohorts_rank_verified_tokens_before_discovery_cost() {
    let mut rows = [1, 11, 12, 13, 14, 15, 16, 17, 18, 19]
        .into_iter()
        .enumerate()
        .map(|(offset, index)| {
            evidence_row(
                index,
                100,
                200,
                K1ConsequenceTypeV1::Collection,
                K1NaturalEvidenceClassV1::NaturalLive,
                if offset < 4 { 300 } else { 301 },
                true,
                offset < 2,
            )
        })
        .collect::<Vec<_>>();
    rows.extend((2..=10).enumerate().map(|(offset, index)| {
        evidence_row(
            index,
            101,
            201,
            K1ConsequenceTypeV1::Collection,
            K1NaturalEvidenceClassV1::NaturalLive,
            if offset < 4 { 302 } else { 303 },
            true,
            offset < 2,
        )
    }));
    let catalog = catalog(&rows);
    let deficit = deficit(vec![K1ConsequenceTypeV1::Scalar]);
    let queue =
        build_k1_natural_candidate_queue_v1(&catalog, &deficit, catalog_watermark(&catalog))
            .expect("queue");

    assert_eq!(queue.rows[0].score.readiness_rank, 1);
    assert_eq!(queue.rows[1].score.readiness_rank, 1);
    assert!(
        queue.rows[0].score.expected_verified_input_tokens
            > queue.rows[1].score.expected_verified_input_tokens
    );
    assert!(
        queue.rows[0].score.bounded_discovery_cost_units
            > queue.rows[1].score.bounded_discovery_cost_units
    );
}

#[test]
fn generated_and_controlled_rows_never_enter_natural_candidates() {
    let rows = vec![
        evidence_row(
            1,
            100,
            200,
            K1ConsequenceTypeV1::Scalar,
            K1NaturalEvidenceClassV1::NaturalLive,
            300,
            true,
            true,
        ),
        evidence_row(
            2,
            100,
            200,
            K1ConsequenceTypeV1::Scalar,
            K1NaturalEvidenceClassV1::GeneratedMs5,
            300,
            true,
            true,
        ),
        evidence_row(
            3,
            100,
            200,
            K1ConsequenceTypeV1::Scalar,
            K1NaturalEvidenceClassV1::GeneratedMs6,
            300,
            true,
            true,
        ),
        evidence_row(
            4,
            100,
            200,
            K1ConsequenceTypeV1::Scalar,
            K1NaturalEvidenceClassV1::Controlled,
            300,
            true,
            true,
        ),
    ];
    let catalog = catalog(&rows);

    assert_eq!(catalog.scanned_rows, 4);
    assert_eq!(catalog.natural_rows, 1);
    assert_eq!(catalog.generated_fixture_rows_excluded, 2);
    assert_eq!(catalog.controlled_rows_excluded, 1);
    assert_eq!(catalog.candidates[0].evidence_rows, 1);
}

#[test]
fn legacy_v1_evidence_is_diagnostic_even_when_its_frozen_hash_was_eligible() {
    let legacy = K1NaturalEvidenceRowV1::seal_legacy_v1(
        root(10_001),
        root(100),
        root(200),
        root(50_100),
        root(300),
        K1ConsequenceTypeV1::Scalar,
        K1NaturalEvidenceClassV1::NaturalLive,
        1,
        1_000,
        1_001,
        true,
        true,
        false,
    )
    .expect("legacy evidence row");
    let catalog = catalog(&[legacy]);

    assert_eq!(catalog.scanned_rows, 1);
    assert_eq!(catalog.safety_veto_rows_excluded, 1);
    assert!(catalog.candidates.is_empty());
}

#[test]
fn oversized_catalog_keeps_a_complete_denominator_and_a_bounded_ready_queue() {
    let mut rows = (1..=256)
        .map(|index| {
            evidence_row(
                index,
                1_000 + index,
                2_000 + index,
                K1ConsequenceTypeV1::Collection,
                K1NaturalEvidenceClassV1::NaturalLive,
                3_000 + index,
                false,
                false,
            )
        })
        .collect::<Vec<_>>();
    rows.extend((400..408).map(|index| {
        evidence_row(
            index,
            9_000,
            9_001,
            K1ConsequenceTypeV1::Scalar,
            K1NaturalEvidenceClassV1::NaturalLive,
            if index < 404 { 9_100 } else { 9_101 },
            true,
            index < 402,
        )
    }));

    let catalog = catalog(&rows);
    let deficit = deficit(vec![K1ConsequenceTypeV1::Scalar]);
    let queue =
        build_k1_natural_candidate_queue_v1(&catalog, &deficit, catalog_watermark(&catalog))
            .expect("bounded queue");
    let ready_candidate = catalog
        .candidates
        .iter()
        .find(|candidate| candidate.readiness.pass)
        .expect("ready candidate");

    assert_eq!(catalog.natural_rows, 264);
    assert_eq!(catalog.candidates.len(), 257);
    assert_eq!(queue.catalog_candidates, 257);
    assert_eq!(queue.completed_candidates_excluded, 0);
    assert_eq!(queue.scored_candidates, 257);
    assert_eq!(queue.capacity_excluded_candidates, 1);
    assert_eq!(queue.rows.len(), 256);
    assert!(queue.readiness_rescue_included);
    assert_eq!(
        queue
            .first_readiness_pass()
            .expect("retained readiness pass")
            .candidate_root_sha256,
        ready_candidate.candidate_root_sha256
    );
}

#[test]
fn motif_catalog_groups_one_exact_law_across_different_ambient_graphs() {
    let small = motif_topology(2, &[(0, 1)]);
    let extended = motif_topology(3, &[(0, 1), (1, 2)]);
    let small_motif = exact_motif(&small, 2, 1);
    let extended_motif = source_neutral_topology_motifs_v1(&extended)
        .expect("extended motifs")
        .into_iter()
        .find(|motif| motif.motif_root_sha256 == small_motif.motif_root_sha256)
        .expect("same exact motif in extended graph");
    assert_ne!(
        small_motif.embeddings[0].ambient_topology_root_sha256,
        extended_motif.embeddings[0].ambient_topology_root_sha256
    );

    let rows = (1..=8)
        .map(|index| {
            motif_evidence_row(
                index,
                if index % 2 == 0 {
                    &small_motif
                } else {
                    &extended_motif
                },
                100,
                if index <= 4 { 300 } else { 301 },
                1_000 + index,
            )
        })
        .collect::<Vec<_>>();
    let catalog = motif_catalog(&rows, &[]);
    let candidate = &catalog.candidates[0];

    assert_eq!(catalog.schema, K1_NATURAL_COHORT_CATALOG_SCHEMA_V2);
    assert_eq!(catalog.candidates.len(), 1);
    assert_eq!(candidate.schema, K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V4);
    assert_eq!(candidate.evidence_rows, 8);
    assert_eq!(candidate.independent_lineages, 2);
    assert!(candidate.readiness.pass);
    assert_eq!(
        candidate.candidate_structural_root_sha256,
        small_motif.motif_root_sha256
    );
    let bytes = serde_json::to_vec(&catalog).expect("encode motif catalog");
    let restored: K1NaturalCohortCatalogV1 =
        serde_json::from_slice(&bytes).expect("decode motif catalog");
    restored
        .validate()
        .expect("validate restored motif catalog");
    assert_eq!(
        serde_json::to_vec(&restored).expect("re-encode motif catalog"),
        bytes
    );
}

#[test]
fn motif_queue_v2_ranks_bounded_cost_before_verified_tokens() {
    let topology = motif_topology(2, &[(0, 1)]);
    let expensive = exact_motif(&topology, 2, 1);
    let cheap = exact_motif(&topology, 1, 0);
    let mut rows = (1..=8)
        .map(|index| {
            motif_evidence_row(
                index,
                &expensive,
                100,
                if index <= 4 { 300 } else { 301 },
                100_000 + index,
            )
        })
        .collect::<Vec<_>>();
    rows.extend((101..=108).map(|index| {
        motif_evidence_row(
            index,
            &cheap,
            100,
            if index <= 104 { 302 } else { 303 },
            1_000 + index,
        )
    }));
    let catalog = motif_catalog(&rows, &[(expensive.motif_root_sha256.clone(), 80)]);
    let queue = build_k1_natural_candidate_queue_v1(&catalog, &deficit(Vec::new()), 108)
        .expect("motif queue");

    assert_eq!(queue.schema, K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V2);
    assert_eq!(queue.rows.len(), 2);
    let first_candidate = catalog
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_root_sha256 == queue.rows[0].candidate_root_sha256)
        .expect("first ranked motif candidate");
    assert_eq!(
        first_candidate.candidate_structural_root_sha256,
        cheap.motif_root_sha256
    );
    assert!(
        queue.rows[0].score.bounded_discovery_cost_units
            < queue.rows[1].score.bounded_discovery_cost_units
    );
    assert!(
        queue.rows[0].score.expected_verified_input_tokens
            < queue.rows[1].score.expected_verified_input_tokens
    );
}

#[test]
fn motif_freeze_v6_seals_every_catalog_and_embedding_root() {
    let topology = motif_topology(2, &[(0, 1)]);
    let motif = exact_motif(&topology, 2, 1);
    let rows = (1..=8)
        .map(|index| {
            motif_evidence_row(
                index,
                &motif,
                100,
                if index <= 4 { 300 } else { 301 },
                1_000 + index,
            )
        })
        .collect::<Vec<_>>();
    let catalog = motif_catalog(&rows, &[]);
    let deficit = deficit(Vec::new());
    let queue =
        build_k1_natural_candidate_queue_v1(&catalog, &deficit, 8).expect("motif candidate queue");
    let queue_row = queue.first_readiness_pass().expect("ready motif");
    let candidate = catalog
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_root_sha256 == queue_row.candidate_root_sha256)
        .expect("motif candidate");
    let freeze = K1NaturalCandidateFreezeV1::seal(
        6,
        &catalog,
        &deficit,
        &queue,
        candidate,
        queue_row.score.clone(),
        "nando.k1-operator-blind-scheduler.v2".to_owned(),
        root(706),
        K1GenerationBudgetV1 {
            maximum_support_rows: 64,
            maximum_probe_rounds: 4,
            maximum_probe_cost_units: 100,
            maximum_generation_seconds: 3_600,
        },
        8,
        8,
        1_700_000_000,
    )
    .expect("motif candidate freeze");

    assert_eq!(freeze.schema, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6);
    assert_eq!(
        freeze.motif_disposition_summary_root_sha256,
        catalog
            .motif_disposition
            .as_ref()
            .expect("motif disposition")
            .summary_root_sha256
    );
    assert_eq!(
        freeze.motif_embedding_manifest_root_sha256,
        candidate.motif_embedding_manifest_root_sha256
    );
    let bytes = serde_json::to_vec(&freeze).expect("encode v6 freeze");
    let restored: K1NaturalCandidateFreezeV1 =
        serde_json::from_slice(&bytes).expect("decode v6 freeze");
    restored.validate().expect("validate restored v6 freeze");
    assert_eq!(
        serde_json::to_vec(&restored).expect("re-encode v6 freeze"),
        bytes
    );

    let validated_catalog =
        ValidatedK1NaturalCohortCatalogV1::try_new(catalog.clone()).expect("validated catalog");
    let optimized_queue = validated_catalog
        .build_candidate_queue_with_exclusions(&deficit, &BTreeSet::new(), 8)
        .expect("optimized motif queue");
    assert_eq!(optimized_queue, queue);
    let optimized_queue_row = optimized_queue
        .first_readiness_pass()
        .expect("optimized ready motif");
    let optimized_candidate = validated_catalog
        .candidates
        .iter()
        .find(|candidate| {
            candidate.candidate_root_sha256 == optimized_queue_row.candidate_root_sha256
        })
        .expect("optimized motif candidate");
    let optimized_freeze = validated_catalog
        .seal_candidate_freeze(
            6,
            &deficit,
            &optimized_queue,
            optimized_candidate,
            optimized_queue_row.score.clone(),
            "nando.k1-operator-blind-scheduler.v2".to_owned(),
            root(706),
            K1GenerationBudgetV1 {
                maximum_support_rows: 64,
                maximum_probe_rounds: 4,
                maximum_probe_cost_units: 100,
                maximum_generation_seconds: 3_600,
            },
            8,
            8,
            1_700_000_000,
        )
        .expect("optimized motif freeze");
    assert_eq!(optimized_freeze, freeze);

    let mut tampered = freeze;
    tampered.motif_embedding_manifest_root_sha256 = root(99_999);
    assert_eq!(
        tampered.validate(),
        Err("k1_natural_candidate_freeze_invalid")
    );
}

#[test]
fn exact_queue_v4_preserves_v2_order_and_only_binds_ready_rows() {
    let topology = motif_topology(2, &[(0, 1)]);
    let ready_motif = exact_motif(&topology, 2, 1);
    let immature_motif = exact_motif(&topology, 1, 0);
    let mut rows = (1..=8)
        .map(|index| {
            motif_evidence_row(
                index,
                &ready_motif,
                100,
                if index <= 4 { 300 } else { 301 },
                1_000 + index,
            )
        })
        .collect::<Vec<_>>();
    rows.push(motif_evidence_row(15, &immature_motif, 101, 302, 1_015));
    let catalog = motif_catalog(&rows, &[]);
    let deficit = deficit(vec![K1ConsequenceTypeV1::Scalar]);
    let queue_v2 =
        build_k1_natural_candidate_queue_v1(&catalog, &deficit, catalog_watermark(&catalog))
            .expect("queue v2");
    let original_order = queue_v2
        .rows
        .iter()
        .map(|row| row.candidate_root_sha256.clone())
        .collect::<Vec<_>>();
    let manifests = queue_v2
        .rows
        .iter()
        .filter(|row| row.score.readiness_rank == 1)
        .map(|row| {
            let candidate = catalog
                .candidates
                .iter()
                .find(|candidate| candidate.candidate_root_sha256 == row.candidate_root_sha256)
                .expect("ready candidate");
            (
                row.candidate_root_sha256.clone(),
                exact_manifest(
                    candidate.candidate_structural_root_sha256.clone(),
                    candidate.last_capture_sequence,
                    root(89_000),
                ),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let queue_v4 = queue_v2
        .bind_exact_opportunities_v4(
            &ExactAttemptIndexV1::empty(7).expect("legacy-only exact index"),
            root(89_001),
            &manifests,
        )
        .expect("queue v4");

    assert_eq!(queue_v4.schema, K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4);
    assert_eq!(
        queue_v4
            .rows
            .iter()
            .map(|row| row.candidate_root_sha256.clone())
            .collect::<Vec<_>>(),
        original_order
    );
    assert_eq!(queue_v4.exact_unseen_opportunities, 1);
    assert_eq!(queue_v4.exact_attempted_deterministic_roots, 0);
    assert_eq!(queue_v4.legacy_unbound_terminals, 7);
    assert!(queue_v4.rows.iter().all(|row| {
        if row.score.readiness_rank == 1 {
            row.exact_attempt_state == "unseen"
                && nando_operator_kernel::valid_nonzero_sha256(&row.causal_manifest_root_sha256)
                && nando_operator_kernel::valid_nonzero_sha256(&row.opportunity_root_sha256)
        } else {
            row.exact_attempt_state.is_empty()
                && row.causal_manifest_root_sha256.is_empty()
                && row.opportunity_root_sha256.is_empty()
        }
    }));
}

#[test]
fn coarse_family_failures_cannot_demote_a_new_exact_opportunity() {
    let topology = motif_topology(2, &[(0, 1)]);
    let motif = exact_motif(&topology, 2, 1);
    let rows = (1..=8)
        .map(|index| {
            motif_evidence_row(
                index,
                &motif,
                100,
                if index <= 4 { 300 } else { 301 },
                1_000 + index,
            )
        })
        .collect::<Vec<_>>();
    let catalog = motif_catalog(&rows, &[]);
    let deficit = deficit(Vec::new());
    let queue_v2 =
        build_k1_natural_candidate_queue_v1(&catalog, &deficit, catalog_watermark(&catalog))
            .expect("queue v2");
    let candidate = &catalog.candidates[0];
    let manifest = exact_manifest(
        candidate.candidate_structural_root_sha256.clone(),
        candidate.last_capture_sequence,
        root(89_010),
    );
    let changed_family_manifest = exact_manifest(
        candidate.candidate_structural_root_sha256.clone(),
        candidate.last_capture_sequence,
        root(89_011),
    );
    let failed_family_attempt = ExactAttemptRecordV1 {
        opportunity_root_sha256: changed_family_manifest.opportunity_root_sha256,
        identifier_result_root_sha256: root(89_012),
        terminal_diagnostic_root_sha256: root(89_013),
        candidate_freeze_root_sha256: root(89_014),
        generation_sequence: 4,
    };
    let index = ExactAttemptIndexV1::seal(vec![failed_family_attempt], 4).expect("exact index");
    let manifests = std::collections::BTreeMap::from([(
        queue_v2.rows[0].candidate_root_sha256.clone(),
        manifest.clone(),
    )]);

    let queue_v4 = queue_v2
        .bind_exact_opportunities_v4(&index, root(89_015), &manifests)
        .expect("queue v4");

    assert_eq!(
        queue_v4.rows[0].opportunity_root_sha256,
        manifest.opportunity_root_sha256
    );
    assert_eq!(queue_v4.rows[0].exact_attempt_state, "unseen");
    assert_eq!(queue_v4.exact_unseen_opportunities, 1);
    assert_eq!(queue_v4.exact_attempted_deterministic_roots, 0);
    assert_eq!(
        queue_v4.first_readiness_pass(),
        queue_v4.rows.first(),
        "only the exact attempted root may suppress a row"
    );
}

#[test]
fn exact_freeze_v8_separates_opportunity_identity_from_provenance() {
    let topology = motif_topology(2, &[(0, 1)]);
    let motif = exact_motif(&topology, 2, 1);
    let rows = (1..=8)
        .map(|index| {
            motif_evidence_row(
                index,
                &motif,
                100,
                if index <= 4 { 300 } else { 301 },
                1_000 + index,
            )
        })
        .collect::<Vec<_>>();
    let catalog = motif_catalog(&rows, &[]);
    let deficit = deficit(Vec::new());
    let queue_v2 = build_k1_natural_candidate_queue_v1(&catalog, &deficit, 8).expect("queue v2");
    let candidate = &catalog.candidates[0];
    let manifest = exact_manifest(
        candidate.candidate_structural_root_sha256.clone(),
        candidate.last_capture_sequence,
        root(89_020),
    );
    let manifests = std::collections::BTreeMap::from([(
        queue_v2.rows[0].candidate_root_sha256.clone(),
        manifest.clone(),
    )]);
    let index = ExactAttemptIndexV1::empty(585).expect("legacy-only exact index");
    let queue_v4 = queue_v2
        .bind_exact_opportunities_v4(&index, root(89_021), &manifests)
        .expect("queue v4");

    let freeze = K1NaturalCandidateFreezeV1::seal_exact_v8(
        586,
        &catalog,
        &deficit,
        &queue_v4,
        candidate,
        queue_v4.rows[0].score.clone(),
        "nando.k1-operator-blind-scheduler.v4".to_owned(),
        root(88_000),
        K1GenerationBudgetV1 {
            maximum_support_rows: 64,
            maximum_probe_rounds: 4,
            maximum_probe_cost_units: 100,
            maximum_generation_seconds: 3_600,
        },
        candidate.last_capture_sequence,
        candidate.last_capture_sequence,
        1_700_000_000,
        manifest.clone(),
        root(89_021),
        root(89_022),
        index.index_root_sha256,
        root(89_023),
    )
    .expect("freeze v8");

    assert_eq!(freeze.schema, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V8);
    assert_ne!(freeze.freeze_root_sha256, manifest.opportunity_root_sha256);
    assert_eq!(
        freeze
            .identifier_causal_input_manifest
            .as_deref()
            .expect("causal manifest")
            .opportunity_root_sha256,
        manifest.opportunity_root_sha256
    );
    let bytes = serde_json::to_vec(&freeze).expect("encode freeze v8");
    let restored: K1NaturalCandidateFreezeV1 =
        serde_json::from_slice(&bytes).expect("decode freeze v8");
    restored.validate().expect("validate restored freeze v8");
    assert_eq!(
        serde_json::to_vec(&restored).expect("re-encode freeze v8"),
        bytes
    );
}
