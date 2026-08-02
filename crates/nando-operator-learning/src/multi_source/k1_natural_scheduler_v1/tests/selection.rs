use super::*;

#[test]
fn queue_can_rank_immature_novel_cohort_first_but_freezes_first_ready_cohort() {
    let mut rows = ready_rows();
    rows.push(evidence_row(
        20,
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
    let queue = build_k1_natural_candidate_queue_v1(&catalog, &deficit).expect("queue");

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
