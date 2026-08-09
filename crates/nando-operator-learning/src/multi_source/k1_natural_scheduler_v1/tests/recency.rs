use super::*;

#[test]
fn unrelated_global_traffic_cannot_stale_a_ready_candidate() {
    let rows = ready_rows();
    let catalog = catalog(&rows);
    let deficit = deficit(Vec::new());
    let candidate = &catalog.candidates[0];
    let unrelated_global_watermark = candidate.last_capture_sequence + 1_000_000;
    let queue = build_k1_natural_candidate_queue_v1(&catalog, &deficit, unrelated_global_watermark)
        .expect("queue");

    assert!(candidate.readiness.pass);
    assert_eq!(queue.rows[0].score.readiness_rank, 1);
    assert!(
        candidate
            .readiness
            .freeze_ready_at(
                candidate.evidence_rows,
                candidate.first_capture_sequence,
                candidate.last_capture_sequence,
                unrelated_global_watermark,
            )
            .expect("cohort-local readiness")
    );

    let queued = queue.first_readiness_pass().expect("ready row");
    K1NaturalCandidateFreezeV1::seal(
        1,
        &catalog,
        &deficit,
        &queue,
        candidate,
        queued.score.clone(),
        "nando.k1-operator-blind-scheduler.v1".to_owned(),
        root(706),
        K1GenerationBudgetV1 {
            maximum_support_rows: 64,
            maximum_probe_rounds: 4,
            maximum_probe_cost_units: 100,
            maximum_generation_seconds: 3_600,
        },
        candidate.last_capture_sequence,
        unrelated_global_watermark,
        1_700_000_000,
    )
    .expect("freeze");
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
        root(706),
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

#[test]
fn insufficient_candidate_remains_blocked_regardless_of_global_traffic() {
    let rows = ready_rows().into_iter().take(7).collect::<Vec<_>>();
    let catalog = catalog(&rows);
    let deficit = deficit(Vec::new());
    let candidate = &catalog.candidates[0];
    let queue = build_k1_natural_candidate_queue_v1(
        &catalog,
        &deficit,
        candidate.last_capture_sequence + 1_000_000,
    )
    .expect("queue");

    assert!(!candidate.readiness.pass);
    assert!(queue.first_readiness_pass().is_none());
    assert_eq!(queue.rows[0].score.readiness_rank, 0);
}
