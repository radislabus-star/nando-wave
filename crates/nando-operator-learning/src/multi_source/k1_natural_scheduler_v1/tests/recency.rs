use super::*;

#[test]
fn stale_count_ready_candidate_cannot_rank_or_freeze() {
    let rows = ready_rows();
    let catalog = catalog(&rows);
    let deficit = deficit(Vec::new());
    let candidate = &catalog.candidates[0];
    let stale_watermark = candidate.last_capture_sequence + candidate.evidence_rows + 1;
    let queue = build_k1_natural_candidate_queue_v1(&catalog, &deficit, stale_watermark)
        .expect("stale queue");

    assert!(candidate.readiness.pass);
    assert_eq!(queue.rows[0].score.readiness_rank, 0);
    assert!(
        !candidate
            .readiness
            .freeze_ready_at(
                candidate.evidence_rows,
                candidate.first_capture_sequence,
                candidate.last_capture_sequence,
                stale_watermark,
            )
            .expect("stale readiness")
    );

    let fresh_watermark = candidate.last_capture_sequence + candidate.evidence_rows;
    let fresh_queue = build_k1_natural_candidate_queue_v1(&catalog, &deficit, fresh_watermark)
        .expect("fresh queue");
    let fresh_row = fresh_queue.first_readiness_pass().expect("fresh row");
    assert_eq!(
        K1NaturalCandidateFreezeV1::seal(
            1,
            &catalog,
            &deficit,
            &fresh_queue,
            candidate,
            fresh_row.score.clone(),
            "nando.k1-operator-blind-scheduler.v1".to_owned(),
            K1GenerationBudgetV1 {
                maximum_support_rows: 64,
                maximum_probe_rounds: 4,
                maximum_probe_cost_units: 100,
                maximum_generation_seconds: 3_600,
            },
            candidate.last_capture_sequence,
            stale_watermark,
            1_700_000_000,
        ),
        Err("k1_candidate_freeze_binding_invalid")
    );
}

#[test]
fn fresh_count_ready_candidate_ranks_and_freezes() {
    let rows = ready_rows();
    let catalog = catalog(&rows);
    let deficit = deficit(Vec::new());
    let candidate = &catalog.candidates[0];
    let fresh_watermark = candidate.last_capture_sequence + candidate.evidence_rows;
    let queue = build_k1_natural_candidate_queue_v1(&catalog, &deficit, fresh_watermark)
        .expect("fresh queue");
    let queued = queue.first_readiness_pass().expect("fresh ready row");

    assert_eq!(queued.score.readiness_rank, 1);
    K1NaturalCandidateFreezeV1::seal(
        1,
        &catalog,
        &deficit,
        &queue,
        candidate,
        queued.score.clone(),
        "nando.k1-operator-blind-scheduler.v1".to_owned(),
        K1GenerationBudgetV1 {
            maximum_support_rows: 64,
            maximum_probe_rounds: 4,
            maximum_probe_cost_units: 100,
            maximum_generation_seconds: 3_600,
        },
        candidate.last_capture_sequence,
        fresh_watermark,
        1_700_000_000,
    )
    .expect("fresh freeze");
}
