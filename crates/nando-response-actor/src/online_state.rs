use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    CegisCoordinator, CegisReport, CegisWinner, CrossSurfaceFamilyDiscovery,
    FROZEN_PARTITION_VERSION, FamilyDiscoveryConfig, FamilyDiscoveryReport, FrozenGeneration,
    OpportunityBoard, OpportunityBoardReport, RolloverPolicy, TeacherPoolSnapshot,
    TeacherTransition, VersionSpaceConfig, freeze_generation, refresh_frozen_generation,
};

pub const SELF_TRAINING_STATE_SCHEMA_V2: &str = "nando.self-training-stream-state.v2";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelfTrainingAdmissionCohort {
    pub winner: CegisWinner,
    pub generation: FrozenGeneration,
    pub pool: TeacherPoolSnapshot,
    pub runtime_parity_cases: Vec<crate::RuntimeParityCase>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelfTrainingGenerationReport {
    pub partition_version: u32,
    pub cohort_id_sha256: String,
    pub teacher_signature_sha256: String,
    pub generation: u64,
    pub support_rows: usize,
    pub support_tokens: u64,
    pub future_rows: usize,
    pub future_tokens: u64,
    pub future_sessions: usize,
    pub surfaces: usize,
    pub wrong_future_rows: usize,
    #[serde(default)]
    pub support_runtime_parity_rows: usize,
    #[serde(default)]
    pub support_runtime_parity_tokens: u64,
    #[serde(default)]
    pub matching_runtime_parity_rows: usize,
    #[serde(default)]
    pub matching_runtime_parity_sessions: usize,
    #[serde(default)]
    pub post_repair_runtime_parity_rows: usize,
    #[serde(default)]
    pub post_repair_runtime_parity_sessions: usize,
    pub runtime_parity_rows: usize,
    pub runtime_parity_tokens: u64,
    pub blocker: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MinerSignalStageReport {
    pub stage: String,
    pub verdict: String,
    pub score_out_of_10: u8,
    pub rows: u64,
    pub blocker: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MinerSignalTreeReport {
    pub overall_score_out_of_10: u8,
    pub stages: Vec<MinerSignalStageReport>,
    pub top_blockers: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelfTrainingStateReport {
    pub schema: String,
    pub transitions_seen: u64,
    pub work_slices_completed: u64,
    pub exact_checks_completed: u64,
    #[serde(default)]
    pub runtime_parity_cases_total: usize,
    pub discovery: FamilyDiscoveryReport,
    pub cegis: CegisReport,
    pub opportunity: OpportunityBoardReport,
    pub generations: Vec<SelfTrainingGenerationReport>,
    pub admission_ready_cohorts: usize,
    pub signal_tree: MinerSignalTreeReport,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StreamingSelfTrainingState {
    schema: String,
    discovery: CrossSurfaceFamilyDiscovery,
    cegis: CegisCoordinator,
    opportunity: OpportunityBoard,
    rollover_policy: RolloverPolicy,
    generations: BTreeMap<String, FrozenGeneration>,
    transitions_seen: u64,
    work_slices_completed: u64,
    exact_checks_completed: u64,
    #[serde(default)]
    negative_refresh_cursor: Option<String>,
    #[serde(default)]
    rebuild_queue: VecDeque<String>,
    #[serde(default)]
    runtime_parity_cases: BTreeMap<String, crate::RuntimeParityCase>,
}

impl StreamingSelfTrainingState {
    #[must_use]
    pub fn new(now_unix: u64) -> Self {
        Self {
            schema: SELF_TRAINING_STATE_SCHEMA_V2.to_owned(),
            discovery: CrossSurfaceFamilyDiscovery::default(),
            cegis: CegisCoordinator::new(VersionSpaceConfig::default(), 16),
            opportunity: OpportunityBoard::new(Default::default(), now_unix),
            rollover_policy: RolloverPolicy::default(),
            generations: BTreeMap::new(),
            transitions_seen: 0,
            work_slices_completed: 0,
            exact_checks_completed: 0,
            negative_refresh_cursor: None,
            rebuild_queue: VecDeque::new(),
            runtime_parity_cases: BTreeMap::new(),
        }
    }

    pub fn observe_migration_transition(
        &mut self,
        transition: &TeacherTransition,
    ) -> Result<(), String> {
        if !self.discovery.observe_transition(transition)? {
            return Ok(());
        }
        self.transitions_seen = self.transitions_seen.saturating_add(1);
        self.opportunity.observe_transition(transition);
        Ok(())
    }

    pub fn prepare_strategy_migration(&mut self) {
        self.discovery
            .enforce_runtime_limits(FamilyDiscoveryConfig::default());
        self.prepare_rebuild(true);
    }

    pub fn prepare_replay_seed(&mut self) {
        self.discovery
            .enforce_runtime_limits(FamilyDiscoveryConfig::default());
        self.prepare_rebuild(false);
    }

    fn prepare_rebuild(&mut self, clear_runtime_parity: bool) {
        self.cegis.prepare_strategy_migration();
        self.generations.clear();
        if clear_runtime_parity {
            self.runtime_parity_cases.clear();
        }
        self.negative_refresh_cursor = None;
        self.rebuild_queue = self
            .discovery
            .pool_snapshots()
            .into_iter()
            .map(|pool| pool.teacher_signature_sha256)
            .collect();
    }

    pub fn prepare_teacher_signature_migration(&mut self) -> Result<(), String> {
        self.discovery
            .enforce_runtime_limits(FamilyDiscoveryConfig::default());
        self.discovery
            .enrich_action_schemas(&self.runtime_parity_cases);
        self.discovery.recanonicalize_teacher_signatures()?;
        self.cegis.prepare_strategy_migration();
        self.generations.clear();
        self.negative_refresh_cursor = None;
        self.rebuild_queue = self
            .discovery
            .pool_snapshots()
            .into_iter()
            .map(|pool| pool.teacher_signature_sha256)
            .collect();
        Ok(())
    }

    pub fn repair_missing_synthesis_state(&mut self) {
        if self.discovery.teacher_pool_count() == 0 || !self.cegis.is_empty() {
            return;
        }
        self.cegis.prepare_strategy_migration();
        self.generations.clear();
        self.negative_refresh_cursor = None;
        self.rebuild_queue = self
            .discovery
            .pool_snapshots()
            .into_iter()
            .map(|pool| pool.teacher_signature_sha256)
            .collect();
    }

    pub fn observe_transition(&mut self, transition: &TeacherTransition) -> Result<(), String> {
        self.observe_runtime_parity_case(transition);
        if !self.discovery.observe_transition(transition)? {
            return Ok(());
        }
        self.transitions_seen = self.transitions_seen.saturating_add(1);
        self.opportunity.observe_transition(transition);
        self.cegis.observe_global_transition(transition);
        let signature = transition.outcome.action.signature_sha256.as_str();
        if let Some(pool) = self.discovery.pool_snapshot(signature) {
            let rows = pool.positive_rows;
            let refresh_due =
                !transition.outcome.verifier.accepted || rows == 16 || (rows > 16 && rows % 8 == 0);
            if refresh_due {
                self.cegis.refresh_pool(&pool);
            }
        }
        if let Some(next_signature) = self
            .cegis
            .next_winner_teacher_signature(self.negative_refresh_cursor.as_deref())
        {
            self.negative_refresh_cursor = Some(next_signature.clone());
            if next_signature != signature
                && let Some(pool) = self.discovery.pool_snapshot(&next_signature)
            {
                self.cegis.refresh_pool(&pool);
            }
        }
        let now_unix = transition.outcome.completed_at_unix_nanos / 1_000_000_000;
        self.opportunity.try_roll_window(now_unix);
        Ok(())
    }

    pub fn observe_runtime_parity_case(&mut self, transition: &TeacherTransition) {
        if let Some(mut parity_case) = transition.runtime_parity_case.clone() {
            let training_frame_id = transition.as_training_relation_frame().frame_id_sha256;
            parity_case.evidence_ref_sha256 = training_frame_id.clone();
            self.runtime_parity_cases
                .insert(training_frame_id, parity_case);
            while self.runtime_parity_cases.len() > 1_024 {
                let Some(oldest) = self.runtime_parity_cases.keys().next().cloned() else {
                    break;
                };
                self.runtime_parity_cases.remove(&oldest);
            }
        }
    }

    /// Continues cold synthesis without requiring another event. The worker
    /// calls this only while queued work exists and always in bounded slices.
    pub fn run_work_slice(&mut self) -> usize {
        if let Some(signature) = self.rebuild_queue.pop_front()
            && let Some(pool) = self.discovery.pool_snapshot(&signature)
        {
            self.cegis.refresh_pool(&pool);
        }
        let checks = self.cegis.run_next_slice();
        if checks > 0 {
            self.work_slices_completed = self.work_slices_completed.saturating_add(1);
            self.exact_checks_completed = self
                .exact_checks_completed
                .saturating_add(u64::try_from(checks).unwrap_or(u64::MAX));
        }
        self.refresh_generations();
        self.refresh_opportunity_search();
        checks
    }

    #[must_use]
    pub fn has_pending_work(&self) -> bool {
        !self.rebuild_queue.is_empty() || self.cegis.has_pending_work()
    }

    #[must_use]
    pub fn admission_cohorts(&self) -> Vec<SelfTrainingAdmissionCohort> {
        if !self.opportunity.authority_safe() {
            return Vec::new();
        }
        let winners = self
            .cegis
            .winners()
            .into_iter()
            .map(|winner| (winner.cohort_id_sha256.clone(), winner))
            .collect::<BTreeMap<_, _>>();
        let mut cohorts = self
            .generations
            .iter()
            .filter(|(_, generation)| generation.blocker.is_none())
            .filter_map(|(cohort_id, generation)| {
                let winner = winners.get(cohort_id)?.clone();
                let pool = self.cohort_pool_snapshot(&winner)?;
                Some(SelfTrainingAdmissionCohort {
                    winner,
                    pool,
                    generation: generation.clone(),
                    runtime_parity_cases: generation
                        .support
                        .iter()
                        .chain(generation.future.iter())
                        .filter_map(|frame| {
                            self.runtime_parity_cases
                                .get(&frame.frame_id_sha256)
                                .cloned()
                        })
                        .collect(),
                })
            })
            .collect::<Vec<_>>();
        cohorts.sort_by(|left, right| {
            let left_tokens = left
                .generation
                .support
                .iter()
                .chain(left.generation.future.iter())
                .map(|frame| frame.estimated_input_tokens)
                .sum::<u64>();
            let right_tokens = right
                .generation
                .support
                .iter()
                .chain(right.generation.future.iter())
                .map(|frame| frame.estimated_input_tokens)
                .sum::<u64>();
            right_tokens.cmp(&left_tokens).then_with(|| {
                left.winner
                    .cohort_id_sha256
                    .cmp(&right.winner.cohort_id_sha256)
            })
        });
        cohorts
    }

    #[must_use]
    pub fn report(&self, now_unix: u64) -> SelfTrainingStateReport {
        let winners = self
            .cegis
            .winners()
            .into_iter()
            .map(|winner| (winner.cohort_id_sha256.clone(), winner))
            .collect::<BTreeMap<_, _>>();
        let parity_diagnostics = winners
            .iter()
            .filter_map(|(cohort_id, winner)| {
                let pool = self.cohort_pool_snapshot(winner)?;
                let matching = pool
                    .positives
                    .iter()
                    .filter(|frame| {
                        self.runtime_parity_cases
                            .contains_key(&frame.frame_id_sha256)
                            && crate::synthesis::program_is_consistent(&winner.program, frame)
                            && frame_has_required_routing_atoms(frame, &winner.required_atom_ids)
                    })
                    .collect::<Vec<_>>();
                let matching_sessions = matching
                    .iter()
                    .map(|frame| frame.session_id_sha256.as_str())
                    .collect::<BTreeSet<_>>()
                    .len();
                let post_repair = matching
                    .iter()
                    .filter(|frame| {
                        frame.observed_at_unix_nanos > winner.repair_watermark_unix_nanos
                    })
                    .copied()
                    .collect::<Vec<_>>();
                let post_repair_sessions = post_repair
                    .iter()
                    .map(|frame| frame.session_id_sha256.as_str())
                    .collect::<BTreeSet<_>>()
                    .len();
                Some((
                    cohort_id.clone(),
                    (
                        matching.len(),
                        matching_sessions,
                        post_repair.len(),
                        post_repair_sessions,
                    ),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        let mut generations = self
            .generations
            .values()
            .map(|generation| {
                let parity = parity_diagnostics
                    .get(&generation.cohort_id_sha256)
                    .copied()
                    .unwrap_or_default();
                SelfTrainingGenerationReport {
                    partition_version: generation.partition_version,
                    cohort_id_sha256: generation.cohort_id_sha256.clone(),
                    teacher_signature_sha256: generation.teacher_signature_sha256.clone(),
                    generation: generation.generation,
                    support_rows: generation.support.len(),
                    support_tokens: generation
                        .support
                        .iter()
                        .map(|frame| frame.estimated_input_tokens)
                        .sum(),
                    future_rows: generation.future.len(),
                    future_tokens: generation
                        .future
                        .iter()
                        .map(|frame| frame.estimated_input_tokens)
                        .sum(),
                    future_sessions: generation.future_sessions,
                    surfaces: generation.surfaces,
                    wrong_future_rows: generation.wrong_future_rows,
                    support_runtime_parity_rows: generation
                        .support
                        .iter()
                        .filter(|frame| {
                            self.runtime_parity_cases
                                .contains_key(&frame.frame_id_sha256)
                        })
                        .count(),
                    support_runtime_parity_tokens: generation
                        .support
                        .iter()
                        .filter(|frame| {
                            self.runtime_parity_cases
                                .contains_key(&frame.frame_id_sha256)
                        })
                        .map(|frame| frame.estimated_input_tokens)
                        .sum(),
                    matching_runtime_parity_rows: parity.0,
                    matching_runtime_parity_sessions: parity.1,
                    post_repair_runtime_parity_rows: parity.2,
                    post_repair_runtime_parity_sessions: parity.3,
                    runtime_parity_rows: generation
                        .future
                        .iter()
                        .filter(|frame| {
                            self.runtime_parity_cases
                                .contains_key(&frame.frame_id_sha256)
                        })
                        .count(),
                    runtime_parity_tokens: generation
                        .future
                        .iter()
                        .filter(|frame| {
                            self.runtime_parity_cases
                                .contains_key(&frame.frame_id_sha256)
                        })
                        .map(|frame| frame.estimated_input_tokens)
                        .sum(),
                    blocker: generation.blocker.clone(),
                }
            })
            .collect::<Vec<_>>();
        generations.sort_by(|left, right| {
            right
                .future_rows
                .cmp(&left.future_rows)
                .then_with(|| left.cohort_id_sha256.cmp(&right.cohort_id_sha256))
        });
        let discovery = self.discovery.report();
        let cegis = self.cegis.report();
        let winner_ids = self
            .cegis
            .winners()
            .into_iter()
            .map(|winner| winner.cohort_id_sha256)
            .collect::<std::collections::BTreeSet<_>>();
        let admission_ready_cohorts = generations
            .iter()
            .filter(|generation| {
                generation.blocker.is_none() && winner_ids.contains(&generation.cohort_id_sha256)
            })
            .count();
        let opportunity = self.opportunity.report(now_unix);
        let signal_tree = build_signal_tree(
            self.transitions_seen,
            &discovery,
            &cegis,
            &generations,
            admission_ready_cohorts,
        );
        SelfTrainingStateReport {
            schema: SELF_TRAINING_STATE_SCHEMA_V2.to_owned(),
            transitions_seen: self.transitions_seen,
            work_slices_completed: self.work_slices_completed,
            exact_checks_completed: self.exact_checks_completed,
            runtime_parity_cases_total: self.runtime_parity_cases.len(),
            discovery,
            cegis,
            opportunity,
            generations,
            admission_ready_cohorts,
            signal_tree,
        }
    }

    pub fn opportunity_mut(&mut self) -> &mut OpportunityBoard {
        &mut self.opportunity
    }

    pub fn observe_ordinary_request(
        &mut self,
        intent_sha256: &str,
        input_tokens: u64,
        now_unix: u64,
    ) {
        self.opportunity
            .observe_request(intent_sha256, input_tokens, true, now_unix);
    }

    pub fn classify_intent(
        &mut self,
        intent_sha256: &str,
        class: crate::ReducibilityClass,
        blocker: Option<&str>,
    ) {
        self.opportunity
            .classify_intent(intent_sha256, class, blocker);
    }

    pub fn mark_verified_intent(&mut self, intent_sha256: &str) {
        self.opportunity.mark_verified(intent_sha256);
    }

    pub fn mark_false_accept(&mut self, intent_sha256: &str) {
        self.opportunity.mark_false_accept(intent_sha256);
    }

    pub fn mark_parity_failure(&mut self, intent_sha256: &str) {
        self.opportunity.mark_parity_failure(intent_sha256);
    }

    #[must_use]
    pub fn teacher_pool_count(&self) -> usize {
        self.discovery.teacher_pool_count()
    }

    #[must_use]
    pub fn admission_ready_cohort_count(&self) -> usize {
        self.admission_cohorts().len()
    }

    fn refresh_generations(&mut self) {
        let winners = self.cegis.winners();
        let future_eligible_ids = self
            .runtime_parity_cases
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let live_ids = winners
            .iter()
            .map(|winner| winner.cohort_id_sha256.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        self.generations
            .retain(|cohort_id, _| live_ids.contains(cohort_id.as_str()));
        for winner in winners {
            let Some(pool) = self.cohort_pool_snapshot(&winner) else {
                continue;
            };
            let generation_number = self
                .generations
                .get(&winner.cohort_id_sha256)
                .map_or(0, |generation| generation.generation);
            let (next, allow_support_repartition) =
                if let Some(current) = self.generations.get(&winner.cohort_id_sha256) {
                    let partition_upgrade = current.partition_version < FROZEN_PARTITION_VERSION;
                    let can_repartition = partition_upgrade
                        || current.support.len() < self.rollover_policy.support_rows;
                    let refrozen = can_repartition.then(|| {
                        freeze_generation(
                            &winner,
                            &pool,
                            self.rollover_policy,
                            generation_number,
                            &future_eligible_ids,
                        )
                    });
                    let repartition_improves = partition_upgrade
                        || refrozen.as_ref().is_some_and(|candidate| {
                            candidate.support.len() >= current.support.len()
                                && candidate.wrong_future_rows <= current.wrong_future_rows
                                && (candidate.support.len() > current.support.len()
                                    || candidate.future.len() > current.future.len()
                                    || candidate.future_sessions > current.future_sessions
                                    || (current.blocker.is_some() && candidate.blocker.is_none()))
                        });
                    if repartition_improves {
                        (refrozen.expect("checked above"), true)
                    } else {
                        (
                            refresh_frozen_generation(
                                current,
                                &winner,
                                &pool,
                                self.rollover_policy,
                                &future_eligible_ids,
                            ),
                            false,
                        )
                    }
                } else {
                    (
                        freeze_generation(
                            &winner,
                            &pool,
                            self.rollover_policy,
                            generation_number,
                            &future_eligible_ids,
                        ),
                        true,
                    )
                };
            let replace = self
                .generations
                .get(&winner.cohort_id_sha256)
                .is_none_or(|current| {
                    allow_support_repartition || generation_evidence_improves(current, &next)
                });
            if replace {
                self.generations
                    .insert(winner.cohort_id_sha256.clone(), next);
            }
        }
    }

    fn cohort_pool_snapshot(&self, winner: &CegisWinner) -> Option<TeacherPoolSnapshot> {
        let mut pool = self
            .discovery
            .pool_snapshot(&winner.teacher_signature_sha256)?;
        let (_, training_negatives) = self.cegis.cohort_evidence(&winner.cohort_id_sha256)?;
        let mut negatives = BTreeMap::<String, crate::RelationFrame>::new();
        for mut frame in pool
            .negatives
            .iter()
            .cloned()
            .chain(training_negatives)
            .chain(
                pool.positives
                    .iter()
                    .filter(|frame| {
                        !crate::synthesis::program_is_consistent(&winner.program, frame)
                    })
                    .cloned(),
            )
        {
            if frame.verifier_label == Some(true)
                && crate::synthesis::program_is_consistent(&winner.program, &frame)
            {
                continue;
            }
            frame.verifier_label = Some(false);
            negatives
                .entry(frame.frame_id_sha256.clone())
                .or_insert(frame);
        }
        pool.negatives = negatives.into_values().collect();
        pool.negative_rows = u64::try_from(pool.negatives.len()).unwrap_or(u64::MAX);
        pool.negative_tokens = pool
            .negatives
            .iter()
            .map(|frame| frame.estimated_input_tokens)
            .sum();
        Some(pool)
    }

    fn refresh_opportunity_search(&mut self) {
        let report = self.cegis.report();
        let mut by_teacher = BTreeMap::<String, (u64, u64, usize, bool, Option<String>)>::new();
        for pool in report.pools {
            let entry = by_teacher
                .entry(pool.teacher_signature_sha256)
                .or_insert((0, 0, 0, false, None));
            entry.0 = entry.0.saturating_add(pool.exact_checks);
            entry.1 = entry.1.saturating_add(pool.search_slices);
            entry.2 = entry.2.saturating_add(pool.ast_nodes);
            entry.3 |= pool.winner;
            if entry.4.is_none() {
                entry.4 = pool.blocker;
            }
        }
        for (teacher_signature, (checks, slices, ast_nodes, winner, blocker)) in by_teacher {
            let transfer_probability = if winner { 1_000 } else { 500 };
            let safe_accept = if blocker.is_none() { 1_000 } else { 0 };
            self.opportunity.observe_search(
                &teacher_signature,
                crate::opportunity::SearchObservation {
                    exact_checks: checks,
                    search_slices: slices,
                    hot_bytes: u64::try_from(ast_nodes.saturating_mul(256)).unwrap_or(u64::MAX),
                    safe_accept_milli: safe_accept,
                    transfer_probability_milli: transfer_probability,
                    blocker: blocker.as_deref(),
                },
            );
        }
    }
}

impl Default for StreamingSelfTrainingState {
    fn default() -> Self {
        Self::new(0)
    }
}

fn frame_has_required_routing_atoms(frame: &crate::RelationFrame, required: &[u64]) -> bool {
    let observed = crate::relation_frame_online_routing_atom_ids(frame);
    required
        .iter()
        .all(|atom| observed.binary_search(atom).is_ok())
}

fn build_signal_tree(
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

fn signal_stage(
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

fn score_ratio(value: u64, target: u64, maximum: u8) -> u8 {
    if target == 0 {
        return maximum;
    }
    u8::try_from(value.min(target).saturating_mul(u64::from(maximum)) / target).unwrap_or(maximum)
}

fn generation_evidence_improves(current: &FrozenGeneration, next: &FrozenGeneration) -> bool {
    let current_support = current
        .support
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let next_support = next
        .support
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    current_support == next_support
        && (next.future.len() > current.future.len()
            || next.future_sessions > current.future_sessions
            || next.surfaces > current.surfaces
            || next.wrong_future_rows > current.wrong_future_rows
            || (current.blocker.is_some() && next.blocker.is_none()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn frame(index: usize) -> crate::RelationFrame {
        crate::RelationFrame {
            schema: crate::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: format!("{index:064x}"),
            event_id_sha256: format!("{:064x}", index + 1_000),
            client_intent_id_sha256: format!("{:064x}", index + 2_000),
            session_id_sha256: format!("{:064x}", index + 3_000),
            observed_at_unix_nanos: u64::try_from(index).unwrap_or(u64::MAX),
            estimated_input_tokens: 1,
            extractor_version: "test".to_owned(),
            verifier_label: Some(true),
            atoms: Vec::new(),
            evidence_ref_sha256: format!("{:064x}", index + 4_000),
        }
    }

    fn generation() -> FrozenGeneration {
        FrozenGeneration {
            partition_version: FROZEN_PARTITION_VERSION,
            generation_id_sha256: "1".repeat(64),
            generation: 0,
            teacher_signature_sha256: "2".repeat(64),
            cohort_id_sha256: "3".repeat(64),
            support: (0..32).map(frame).collect(),
            future: Vec::new(),
            negatives: Vec::new(),
            support_watermark_unix_nanos: 31,
            support_sessions: 32,
            future_sessions: 0,
            surfaces: 2,
            wrong_future_rows: 0,
            blocker: Some("future_rows_below_32".to_owned()),
        }
    }

    #[test]
    fn generation_evidence_never_trades_immutable_support_for_future() {
        let current = generation();
        let mut degraded = current.clone();
        degraded.support.pop();
        degraded.future = (100..164).map(frame).collect();
        assert!(!generation_evidence_improves(&current, &degraded));

        let mut improved = current.clone();
        improved.future.push(frame(100));
        assert!(generation_evidence_improves(&current, &improved));
    }

    #[test]
    fn runtime_parity_is_keyed_by_the_frozen_training_frame() {
        let transition = crate::TeacherTransition {
            schema: crate::TEACHER_TRANSITION_SCHEMA_V1.to_owned(),
            before: crate::RuntimeFrame {
                schema: crate::RUNTIME_FRAME_SCHEMA_V1.to_owned(),
                frame_id_sha256: "a".repeat(64),
                event_id_sha256: "b".repeat(64),
                client_intent_id_sha256: "c".repeat(64),
                session_id_sha256: "d".repeat(64),
                observed_at_unix_nanos: 1,
                extractor_version: "test".to_owned(),
                atoms: Vec::new(),
                evidence_ref_sha256: "e".repeat(64),
            },
            outcome: crate::TeacherOutcome {
                schema: crate::TEACHER_OUTCOME_SCHEMA_V1.to_owned(),
                action: crate::TeacherActionAst {
                    signature_sha256: "f".repeat(64),
                    action_symbol: "function:wait".to_owned(),
                    atoms: Vec::new(),
                },
                verifier: crate::TeacherVerifierEvidence {
                    accepted: true,
                    evidence_ref_sha256: "1".repeat(64),
                    output_digest_sha256: "2".repeat(64),
                },
                completed_at_unix_nanos: 2,
            },
            economics: None,
            runtime_parity_case: Some(crate::RuntimeParityCase {
                evidence_ref_sha256: "old".to_owned(),
                request_text: "continue".to_owned(),
                provider_payload: json!({"input": []}),
                expected_response: "{}".to_owned(),
            }),
        };
        let training_frame_id = transition.as_training_relation_frame().frame_id_sha256;
        let mut state = StreamingSelfTrainingState::default();
        state.observe_runtime_parity_case(&transition);

        let parity = state
            .runtime_parity_cases
            .get(&training_frame_id)
            .expect("training-frame parity");
        assert_eq!(parity.evidence_ref_sha256, training_frame_id);
        assert!(
            !state
                .runtime_parity_cases
                .contains_key(&transition.before.frame_id_sha256)
        );
    }
}
