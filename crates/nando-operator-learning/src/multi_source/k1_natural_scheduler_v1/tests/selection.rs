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
