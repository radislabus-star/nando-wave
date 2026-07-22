use super::*;

pub(super) fn build_signal_tree(
    transitions_seen: u64,
    discovery: &FamilyDiscoveryReport,
    cegis: &CegisReport,
    generations: &[SelfTrainingGenerationReport],
    admission_ready_cohorts: usize,
) -> MinerSignalTreeReport {
    let best_generation = generations.iter().max_by_key(|generation| {
        (
            generation.blocker.is_none(),
            generation.future_sessions,
            generation.future_rows,
            generation.support_rows,
        )
    });
    let max_future = best_generation.map_or(0, |generation| generation.future_rows);
    let frozen_future_blocker = best_generation.map_or_else(
        || Some("no_frozen_generation".to_owned()),
        |generation| generation.blocker.clone(),
    );
    let phase_invariants = discovery.invariant_candidates.max(
        cegis
            .pools
            .iter()
            .filter(|pool| pool.winner)
            .map(|pool| pool.invariant_count)
            .sum(),
    );
    let program_families = discovery
        .teacher_pools
        .iter()
        .map(|pool| pool.action_symbol.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let mut top_blockers = BTreeMap::<String, usize>::new();
    for blocker in cegis
        .pools
        .iter()
        .filter_map(|pool| pool.blocker.as_ref())
        .chain(
            generations
                .iter()
                .filter_map(|generation| generation.blocker.as_ref()),
        )
    {
        *top_blockers.entry(blocker.clone()).or_default() += 1;
    }
    if cegis.pools_waiting_after_repair > 0 {
        top_blockers.insert(
            "waiting_for_post_counterexample_support".to_owned(),
            cegis.pools_waiting_after_repair,
        );
    }
    let stages = vec![
        signal_stage(
            "capture",
            transitions_seen,
            score_ratio(transitions_seen, 32, 10),
            (transitions_seen == 0).then(|| "no_teacher_transitions".to_owned()),
        ),
        signal_stage(
            "teacher_grouping",
            u64::try_from(discovery.teacher_pool_count).unwrap_or(u64::MAX),
            score_ratio(
                u64::try_from(discovery.teacher_pool_count).unwrap_or(u64::MAX),
                5,
                10,
            ),
            (discovery.teacher_pool_count < 5).then(|| {
                format!(
                    "teacher_program_pools_below_5:{}",
                    discovery.teacher_pool_count
                )
            }),
        ),
        signal_stage(
            "phase_invariants",
            u64::try_from(phase_invariants).unwrap_or(u64::MAX),
            score_ratio(u64::try_from(phase_invariants).unwrap_or(u64::MAX), 8, 10),
            (phase_invariants < 8).then(|| format!("phase_invariants_below_8:{phase_invariants}")),
        ),
        signal_stage(
            "typed_synthesis",
            u64::try_from(program_families).unwrap_or(u64::MAX),
            score_ratio(u64::try_from(program_families).unwrap_or(u64::MAX), 5, 10),
            (program_families < 5)
                .then(|| format!("typed_program_families_below_5:{program_families}")),
        ),
        signal_stage(
            "cegis",
            u64::try_from(cegis.winners).unwrap_or(u64::MAX),
            score_ratio(u64::try_from(cegis.winners).unwrap_or(u64::MAX), 5, 10),
            (cegis.winners < 5).then(|| {
                if cegis.pools_waiting_after_repair > 0 {
                    format!(
                        "post_counterexample_support_pending:{}",
                        cegis.pools_waiting_after_repair
                    )
                } else {
                    format!("cegis_winners_below_5:{}", cegis.winners)
                }
            }),
        ),
        signal_stage(
            "frozen_future",
            u64::try_from(max_future).unwrap_or(u64::MAX),
            if frozen_future_blocker.is_none() {
                10
            } else {
                score_ratio(u64::try_from(max_future).unwrap_or(u64::MAX), 32, 9)
            },
            frozen_future_blocker,
        ),
        signal_stage(
            "candidate_ready_for_external_admission",
            u64::try_from(admission_ready_cohorts).unwrap_or(u64::MAX),
            score_ratio(
                u64::try_from(admission_ready_cohorts).unwrap_or(u64::MAX),
                4,
                10,
            ),
            (admission_ready_cohorts < 4)
                .then(|| format!("admission_ready_cohorts_below_4:{admission_ready_cohorts}")),
        ),
    ];
    let overall_score_out_of_10 = stages
        .iter()
        .map(|stage| stage.score_out_of_10)
        .min()
        .unwrap_or(0);
    MinerSignalTreeReport {
        overall_score_out_of_10,
        stages,
        top_blockers,
    }
}

pub(super) fn signal_stage(
    stage: &str,
    rows: u64,
    score_out_of_10: u8,
    blocker: Option<String>,
) -> MinerSignalStageReport {
    MinerSignalStageReport {
        stage: stage.to_owned(),
        verdict: if blocker.is_none() {
            "PASS".to_owned()
        } else if rows > 0 {
            "WATCH".to_owned()
        } else {
            "BLOCK".to_owned()
        },
        score_out_of_10: score_out_of_10.min(10),
        rows,
        blocker,
    }
}

pub(super) fn score_ratio(value: u64, target: u64, maximum: u8) -> u8 {
    if target == 0 {
        return maximum;
    }
    u8::try_from(value.min(target).saturating_mul(u64::from(maximum)) / target).unwrap_or(maximum)
}
