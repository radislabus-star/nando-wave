use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::teacher_join::program_has_compatible_teacher_effect;
use crate::{
    ResponseOperation, ResponseProgram, RuntimeInvariant, TeacherPoolSnapshot, VersionSpaceArena,
    VersionSpaceConfig, ground_roles, relation_frame_online_routing_atom_ids,
};

const MAX_ACTIVE_CEGIS_STATES: usize = 128;
const MAX_APPLICABILITY_ATOMS: usize = 64;
const MAX_APPLICABILITY_SUBCENTERS: usize = 16;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterexampleKind {
    SelectorAmbiguity,
    WrongRoleBinding,
    SameActionDifferentProgram,
    CompetingActionAccept,
    LayoutFailure,
    StaleObservation,
    DuplicateExecutionRisk,
    OutputMismatch,
    VerifierUnavailable,
    NoCleanPreActionInvariant,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairAction {
    StrengthenGuard,
    ReplaceSelector,
    RegroundRoles,
    AddAntiCenter,
    SplitTeacherPool,
    RepairRenderer,
    AddFreshnessGuard,
    AddIdempotencyGuard,
    RejectHypothesis,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CegisCounterexample {
    pub frame_id_sha256: String,
    pub kind: CounterexampleKind,
    pub repair: RepairAction,
    pub eliminated_program_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CegisWinner {
    pub cohort_id_sha256: String,
    pub teacher_signature_sha256: String,
    pub action_symbol: String,
    pub program: ResponseProgram,
    pub required_atom_ids: Vec<u64>,
    pub positive_rows: usize,
    pub negative_rows: usize,
    pub exact_checks: u64,
    pub search_slices: u64,
    pub phase_rank: u32,
    #[serde(default)]
    pub support_frame_ids: Vec<String>,
    #[serde(default)]
    pub support_watermark_unix_nanos: u64,
    #[serde(default)]
    pub repair_watermark_unix_nanos: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CegisPoolReport {
    pub cohort_id_sha256: String,
    pub teacher_signature_sha256: String,
    pub positive_rows: usize,
    pub negative_rows: usize,
    pub invariant_count: usize,
    pub ast_nodes: usize,
    pub survivors: usize,
    pub exact_checks: u64,
    pub search_slices: u64,
    pub counterexamples: usize,
    #[serde(default)]
    pub generated_repair_programs: u64,
    pub winner: bool,
    pub repair_watermark_unix_nanos: u64,
    pub blocker: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CegisReport {
    pub teacher_pools: usize,
    pub cohorts: usize,
    pub winners: usize,
    pub exact_checks: u64,
    pub search_slices: u64,
    pub counterexamples: usize,
    pub repair_events: u64,
    #[serde(default)]
    pub generated_repair_programs: u64,
    pub pools_waiting_after_repair: usize,
    pub state_capacity_evictions: u64,
    pub pools: Vec<CegisPoolReport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ActiveEvaluation {
    program: crate::InternedProgram,
    positive_index: usize,
    #[serde(default)]
    negative_index: usize,
    #[serde(default)]
    invariant_index: usize,
    #[serde(default)]
    phase_delegated: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CegisSearchState {
    cohort_id_sha256: String,
    teacher_signature_sha256: String,
    action_symbol: String,
    evidence_digest_sha256: String,
    #[serde(default)]
    repair_watermark_unix_nanos: u64,
    positives: Vec<crate::RelationFrame>,
    negatives: Vec<crate::RelationFrame>,
    invariants: Vec<RuntimeInvariant>,
    arena: VersionSpaceArena,
    active: Option<ActiveEvaluation>,
    winner: Option<CegisWinner>,
    counterexamples: Vec<CegisCounterexample>,
    #[serde(default)]
    generated_repair_programs: u64,
    blocker: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CegisCoordinator {
    version_space_config: VersionSpaceConfig,
    minimum_cohort_rows: usize,
    states: BTreeMap<String, CegisSearchState>,
    next_state_cursor: usize,
    #[serde(default)]
    teacher_repair_watermarks: BTreeMap<String, u64>,
    #[serde(default)]
    repair_events: u64,
    #[serde(default)]
    state_capacity_evictions: u64,
}

impl CegisCoordinator {
    #[must_use]
    pub fn new(version_space_config: VersionSpaceConfig, minimum_cohort_rows: usize) -> Self {
        Self {
            version_space_config,
            minimum_cohort_rows: minimum_cohort_rows.max(1),
            states: BTreeMap::new(),
            next_state_cursor: 0,
            teacher_repair_watermarks: BTreeMap::new(),
            repair_events: 0,
            state_capacity_evictions: 0,
        }
    }

    pub fn prepare_strategy_migration(&mut self) {
        self.version_space_config = VersionSpaceConfig::default();
        self.states.clear();
        self.next_state_cursor = 0;
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn refresh_pool(&mut self, pool: &TeacherPoolSnapshot) {
        self.invalidate_from_new_counterexample(pool);
        let repair_watermark_unix_nanos = self
            .teacher_repair_watermarks
            .get(&pool.teacher_signature_sha256)
            .copied()
            .unwrap_or(0);
        let cohorts =
            discover_program_cohorts(pool, self.minimum_cohort_rows, repair_watermark_unix_nanos);
        let live_ids = cohorts
            .iter()
            .map(|cohort| cohort.cohort_id_sha256.clone())
            .collect::<BTreeSet<_>>();
        self.states.retain(|_, state| {
            state.teacher_signature_sha256 != pool.teacher_signature_sha256
                || live_ids.contains(&state.cohort_id_sha256)
        });
        for cohort in cohorts {
            if self
                .states
                .get(&cohort.cohort_id_sha256)
                .is_some_and(|state| {
                    state.winner.is_some()
                        && state.positives.len() >= 32
                        && state.repair_watermark_unix_nanos == repair_watermark_unix_nanos
                })
            {
                continue;
            }
            let evidence_digest_sha256 = crate::sha256_bytes(
                &serde_json::to_vec(&(
                    cohort
                        .positives
                        .iter()
                        .map(|frame| frame.frame_id_sha256.as_str())
                        .collect::<Vec<_>>(),
                    cohort
                        .negatives
                        .iter()
                        .map(|frame| frame.frame_id_sha256.as_str())
                        .collect::<Vec<_>>(),
                    &cohort.invariants,
                ))
                .unwrap_or_default(),
            );
            if self
                .states
                .get(&cohort.cohort_id_sha256)
                .is_some_and(|state| state.evidence_digest_sha256 == evidence_digest_sha256)
            {
                continue;
            }
            let mut arena = VersionSpaceArena::new(self.version_space_config);
            arena.intern_all(cohort.programs);
            arena.rank_for_support(&cohort.positives);
            let blocker = candidate_invariants(&cohort.invariants, &cohort.positives)
                .is_empty()
                .then(|| "no_clean_pre_action_invariant".to_owned());
            let mut counterexamples = Vec::new();
            if blocker.is_some() {
                counterexamples.push(CegisCounterexample {
                    frame_id_sha256: cohort
                        .negatives
                        .first()
                        .map_or_else(String::new, |frame| frame.frame_id_sha256.clone()),
                    kind: CounterexampleKind::NoCleanPreActionInvariant,
                    repair: RepairAction::SplitTeacherPool,
                    eliminated_program_sha256: None,
                });
            }
            self.states.insert(
                cohort.cohort_id_sha256.clone(),
                CegisSearchState {
                    cohort_id_sha256: cohort.cohort_id_sha256,
                    teacher_signature_sha256: pool.teacher_signature_sha256.clone(),
                    action_symbol: pool.action_symbol.clone(),
                    evidence_digest_sha256,
                    repair_watermark_unix_nanos,
                    positives: cohort.positives,
                    negatives: cohort.negatives,
                    invariants: cohort.invariants,
                    arena,
                    active: None,
                    winner: None,
                    counterexamples,
                    generated_repair_programs: 0,
                    blocker,
                },
            );
        }
        self.enforce_state_capacity();
    }

    /// Scores one completed teacher row against every current winner before
    /// that delayed label can update a pool. Routed disagreement revokes the
    /// winner immediately and starts a fresh post-counterexample generation.
    pub fn observe_global_transition(&mut self, transition: &crate::TeacherTransition) -> usize {
        let frame = transition.as_training_relation_frame();
        let signature = transition.outcome.action.signature_sha256.as_str();
        let accepted = transition.outcome.verifier.accepted;
        let mut invalidated = BTreeMap::<String, u64>::new();
        for state in self.states.values() {
            let Some(winner) = state.winner.as_ref() else {
                continue;
            };
            let after = winner
                .support_watermark_unix_nanos
                .max(winner.repair_watermark_unix_nanos);
            if frame.observed_at_unix_nanos <= after
                || !frame_has_atoms(&frame, &winner.required_atom_ids)
                || !crate::synthesis::program_runtime_applicable(&winner.program, &frame)
            {
                continue;
            }
            let agrees = accepted
                && ((state.teacher_signature_sha256 == signature
                    && crate::synthesis::program_is_consistent(&winner.program, &frame))
                    || program_has_compatible_teacher_effect(&winner.program, &frame));
            if !agrees {
                invalidated
                    .entry(state.teacher_signature_sha256.clone())
                    .and_modify(|watermark| {
                        *watermark = (*watermark).min(frame.observed_at_unix_nanos);
                    })
                    .or_insert(frame.observed_at_unix_nanos);
            }
        }
        if invalidated.is_empty() {
            return 0;
        }
        for (teacher_signature, watermark) in &invalidated {
            self.teacher_repair_watermarks
                .entry(teacher_signature.clone())
                .and_modify(|existing| *existing = (*existing).max(*watermark))
                .or_insert(*watermark);
        }
        self.repair_events = self
            .repair_events
            .saturating_add(u64::try_from(invalidated.len()).unwrap_or(u64::MAX));
        self.states
            .retain(|_, state| !invalidated.contains_key(&state.teacher_signature_sha256));
        self.next_state_cursor = 0;
        invalidated.len()
    }

    /// Runs at most one bounded exact-execution slice and persists its cursor.
    pub fn run_next_slice(&mut self) -> usize {
        let keys = self.states.keys().cloned().collect::<Vec<_>>();
        if keys.is_empty() {
            return 0;
        }
        for offset in 0..keys.len() {
            let index = self.next_state_cursor.saturating_add(offset) % keys.len();
            let key = &keys[index];
            let state = self.states.get_mut(key).expect("CEGIS state exists");
            if state.winner.is_some() || state.blocker.is_some() {
                continue;
            }
            self.next_state_cursor = index.saturating_add(1) % keys.len();
            return run_state_slice(state);
        }
        0
    }

    #[must_use]
    pub fn has_pending_work(&self) -> bool {
        self.states.values().any(|state| {
            state.winner.is_none()
                && state.blocker.as_deref() != Some("no_clean_pre_action_invariant")
                && (state.active.is_some() || state.arena.has_pending_candidates())
        })
    }

    #[must_use]
    pub fn winners(&self) -> Vec<CegisWinner> {
        let mut winners = self
            .states
            .values()
            .filter_map(|state| state.winner.clone())
            .collect::<Vec<_>>();
        winners.sort_by(|left, right| {
            right
                .positive_rows
                .cmp(&left.positive_rows)
                .then_with(|| left.cohort_id_sha256.cmp(&right.cohort_id_sha256))
        });
        winners
    }

    #[must_use]
    pub fn cohort_evidence(
        &self,
        cohort_id_sha256: &str,
    ) -> Option<(Vec<crate::RelationFrame>, Vec<crate::RelationFrame>)> {
        self.states
            .get(cohort_id_sha256)
            .map(|state| (state.positives.clone(), state.negatives.clone()))
    }

    #[must_use]
    pub fn next_winner_teacher_signature(&self, after: Option<&str>) -> Option<String> {
        let signatures = self
            .states
            .values()
            .filter(|state| state.winner.is_some())
            .map(|state| state.teacher_signature_sha256.as_str())
            .collect::<BTreeSet<_>>();
        after
            .and_then(|cursor| {
                signatures
                    .iter()
                    .find(|signature| **signature > cursor)
                    .map(|signature| (*signature).to_owned())
            })
            .or_else(|| signatures.first().map(|signature| (*signature).to_owned()))
    }

    #[must_use]
    pub fn report(&self) -> CegisReport {
        let mut report = CegisReport::default();
        let mut teacher_pools = BTreeSet::new();
        for state in self.states.values() {
            teacher_pools.insert(state.teacher_signature_sha256.as_str());
            let arena = state.arena.report();
            report.exact_checks = report.exact_checks.saturating_add(arena.exact_checks);
            report.search_slices = report.search_slices.saturating_add(arena.slices_completed);
            report.counterexamples = report
                .counterexamples
                .saturating_add(state.counterexamples.len());
            report.generated_repair_programs = report
                .generated_repair_programs
                .saturating_add(state.generated_repair_programs);
            report.winners = report
                .winners
                .saturating_add(usize::from(state.winner.is_some()));
            report.pools.push(CegisPoolReport {
                cohort_id_sha256: state.cohort_id_sha256.clone(),
                teacher_signature_sha256: state.teacher_signature_sha256.clone(),
                positive_rows: state.positives.len(),
                negative_rows: state.negatives.len(),
                invariant_count: state.invariants.len(),
                ast_nodes: arena.ast_nodes,
                survivors: arena.survivors,
                exact_checks: arena.exact_checks,
                search_slices: arena.slices_completed,
                counterexamples: state.counterexamples.len(),
                generated_repair_programs: state.generated_repair_programs,
                winner: state.winner.is_some(),
                repair_watermark_unix_nanos: state.repair_watermark_unix_nanos,
                blocker: state.blocker.clone(),
            });
        }
        report.repair_events = self.repair_events;
        report.state_capacity_evictions = self.state_capacity_evictions;
        report.pools_waiting_after_repair = self
            .teacher_repair_watermarks
            .keys()
            .filter(|signature| {
                !self.states.values().any(|state| {
                    state.teacher_signature_sha256 == signature.as_str() && state.winner.is_some()
                })
            })
            .count();
        report.teacher_pools = teacher_pools.len();
        report.cohorts = self.states.len();
        report.pools.sort_by(|left, right| {
            right
                .positive_rows
                .cmp(&left.positive_rows)
                .then_with(|| left.cohort_id_sha256.cmp(&right.cohort_id_sha256))
        });
        report
    }

    fn invalidate_from_new_counterexample(&mut self, pool: &TeacherPoolSnapshot) {
        let current_repair_watermark = self
            .teacher_repair_watermarks
            .get(&pool.teacher_signature_sha256)
            .copied()
            .unwrap_or(0);
        let mut first_counterexample = None::<(u64, String)>;
        for state in self
            .states
            .values()
            .filter(|state| state.teacher_signature_sha256 == pool.teacher_signature_sha256)
        {
            let Some(winner) = state.winner.as_ref() else {
                continue;
            };
            let support_watermark = winner.support_watermark_unix_nanos.max(
                state
                    .positives
                    .iter()
                    .filter(|frame| winner.support_frame_ids.contains(&frame.frame_id_sha256))
                    .map(|frame| frame.observed_at_unix_nanos)
                    .max()
                    .unwrap_or(0),
            );
            let after = current_repair_watermark.max(support_watermark);
            for frame in
                pool.negatives
                    .iter()
                    .chain(pool.positives.iter().filter(|frame| {
                        !crate::synthesis::program_is_consistent(&winner.program, frame)
                    }))
            {
                if frame.verifier_label == Some(true)
                    && crate::synthesis::program_is_consistent(&winner.program, frame)
                {
                    continue;
                }
                if frame.observed_at_unix_nanos <= after
                    || !frame_has_atoms(frame, &winner.required_atom_ids)
                    || !crate::synthesis::program_runtime_applicable(&winner.program, frame)
                {
                    continue;
                }
                let candidate = (frame.observed_at_unix_nanos, frame.frame_id_sha256.clone());
                if first_counterexample
                    .as_ref()
                    .is_none_or(|current| candidate < *current)
                {
                    first_counterexample = Some(candidate);
                }
            }
        }
        let Some((watermark, _)) = first_counterexample else {
            return;
        };
        self.teacher_repair_watermarks
            .insert(pool.teacher_signature_sha256.clone(), watermark);
        self.repair_events = self.repair_events.saturating_add(1);
        self.states
            .retain(|_, state| state.teacher_signature_sha256 != pool.teacher_signature_sha256);
        self.next_state_cursor = 0;
    }

    fn enforce_state_capacity(&mut self) {
        if self.states.len() <= MAX_ACTIVE_CEGIS_STATES {
            return;
        }
        let mut ranked = self
            .states
            .iter()
            .map(|(id, state)| {
                let positive_tokens = state
                    .positives
                    .iter()
                    .map(|frame| frame.estimated_input_tokens)
                    .sum::<u64>();
                (
                    id.clone(),
                    state.winner.is_some(),
                    positive_tokens,
                    state.positives.len(),
                )
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| right.3.cmp(&left.3))
                .then_with(|| left.0.cmp(&right.0))
        });
        let retained = ranked
            .into_iter()
            .take(MAX_ACTIVE_CEGIS_STATES)
            .map(|entry| entry.0)
            .collect::<BTreeSet<_>>();
        let before = self.states.len();
        self.states.retain(|id, _| retained.contains(id));
        self.state_capacity_evictions = self.state_capacity_evictions.saturating_add(
            u64::try_from(before.saturating_sub(self.states.len())).unwrap_or(u64::MAX),
        );
        self.next_state_cursor %= self.states.len().max(1);
    }
}

impl Default for CegisCoordinator {
    fn default() -> Self {
        Self::new(VersionSpaceConfig::default(), 16)
    }
}

#[derive(Clone)]
struct ProgramCohort {
    cohort_id_sha256: String,
    positives: Vec<crate::RelationFrame>,
    negatives: Vec<crate::RelationFrame>,
    invariants: Vec<RuntimeInvariant>,
    programs: Vec<ResponseProgram>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ApplicabilitySubcenter {
    atom_ids: Vec<u64>,
    positive_rows: usize,
    positive_tokens: u64,
}

fn discover_program_cohorts(
    pool: &TeacherPoolSnapshot,
    minimum_rows: usize,
    repair_watermark_unix_nanos: u64,
) -> Vec<ProgramCohort> {
    let mut discovery_support = pool.positives.to_vec();
    discovery_support.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });
    discovery_support.truncate(32);
    let mut programs = crate::synthesis::enumerate_response_program_candidates(&discovery_support);
    for positive in &discovery_support {
        programs.extend(crate::synthesis::enumerate_response_program_candidates(
            std::slice::from_ref(positive),
        ));
    }
    let mut unique_programs = BTreeMap::<String, ResponseProgram>::new();
    for program in programs {
        let digest = crate::sha256_bytes(&serde_json::to_vec(&program).unwrap_or_default());
        unique_programs.entry(digest).or_insert(program);
    }
    if discovery_support.len() < minimum_rows || unique_programs.is_empty() {
        return Vec::new();
    }
    let programs = unique_programs.into_iter().collect::<Vec<_>>();
    let coverage = programs
        .iter()
        .filter_map(|(digest, program)| {
            let indices = discovery_support
                .iter()
                .enumerate()
                .filter_map(|(index, frame)| {
                    crate::synthesis::program_is_consistent(program, frame).then_some(index)
                })
                .collect::<Vec<_>>();
            (indices.len() >= minimum_rows).then(|| (digest.clone(), indices))
        })
        .collect::<Vec<_>>();
    let mut uncovered = (0..discovery_support.len()).collect::<BTreeSet<_>>();
    let mut cohorts = Vec::new();
    while !uncovered.is_empty() && cohorts.len() < 64 {
        let mut ranked = coverage
            .iter()
            .filter_map(|(digest, indices)| {
                let newly_covered = indices
                    .iter()
                    .filter(|index| uncovered.contains(index))
                    .copied()
                    .collect::<Vec<_>>();
                if newly_covered.is_empty() {
                    return None;
                }
                let new_tokens = newly_covered
                    .iter()
                    .filter_map(|index| discovery_support.get(*index))
                    .map(|frame| frame.estimated_input_tokens)
                    .sum::<u64>();
                Some((digest, indices, newly_covered.len(), new_tokens))
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .2
                .cmp(&left.2)
                .then_with(|| right.3.cmp(&left.3))
                .then_with(|| right.1.len().cmp(&left.1.len()))
                .then_with(|| left.0.cmp(right.0))
        });
        let Some((selected_digest, selected_indices, _, _)) = ranked.first().copied() else {
            break;
        };
        let Some(selected_program) = programs
            .iter()
            .find_map(|(digest, program)| (digest == selected_digest).then_some(program))
        else {
            break;
        };
        let mut all_positives = pool.positives.to_vec();
        all_positives.sort_by(|left, right| {
            left.observed_at_unix_nanos
                .cmp(&right.observed_at_unix_nanos)
                .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
        });
        let discovery_watermark = discovery_support
            .iter()
            .map(|frame| frame.observed_at_unix_nanos)
            .max()
            .unwrap_or(0)
            .max(repair_watermark_unix_nanos);
        let discovery_negatives = dedupe_frames(
            pool.negatives
                .iter()
                .filter(|frame| {
                    frame.observed_at_unix_nanos <= discovery_watermark
                        && (frame.verifier_label != Some(true)
                            || !crate::synthesis::program_is_consistent(selected_program, frame))
                })
                .cloned()
                .chain(
                    discovery_support
                        .iter()
                        .filter(|frame| {
                            !crate::synthesis::program_is_consistent(selected_program, frame)
                        })
                        .map(|frame| {
                            let mut negative = frame.clone();
                            negative.verifier_label = Some(false);
                            negative
                        }),
                ),
        );
        let mut applicability_subcenters = discover_clean_applicability_subcenters(
            selected_program,
            &discovery_support,
            &discovery_negatives,
            minimum_rows,
            MAX_APPLICABILITY_SUBCENTERS,
        );
        let indistinguishable_negative_vectors = discovery_negatives
            .iter()
            .map(relation_frame_online_routing_atom_ids)
            .collect::<BTreeSet<_>>();
        applicability_subcenters.insert(
            0,
            ApplicabilitySubcenter {
                atom_ids: Vec::new(),
                positive_rows: discovery_support.len(),
                positive_tokens: discovery_support
                    .iter()
                    .map(|frame| frame.estimated_input_tokens)
                    .sum(),
            },
        );
        for subcenter in applicability_subcenters {
            if cohorts.len() >= 64 {
                break;
            }
            let matching_positives = all_positives
                .iter()
                .filter(|frame| {
                    crate::synthesis::program_is_consistent(selected_program, frame)
                        && frame_has_atoms(frame, &subcenter.atom_ids)
                        && !indistinguishable_negative_vectors
                            .contains(&relation_frame_online_routing_atom_ids(frame))
                })
                .cloned()
                .collect::<Vec<_>>();
            if matching_positives.len() < minimum_rows {
                continue;
            }
            let rollover_policy = crate::RolloverPolicy::default();
            let support_sessions = crate::rollover::select_support_sessions(
                &matching_positives,
                rollover_policy.support_rows,
                rollover_policy.minimum_future_sessions,
            );
            let support_candidates = matching_positives
                .iter()
                .filter(|frame| support_sessions.contains(frame.session_id_sha256.as_str()))
                .take(rollover_policy.support_rows.saturating_mul(2))
                .cloned()
                .collect::<Vec<_>>();
            let positives = crate::rollover::select_diverse_support_rows(
                &support_candidates,
                rollover_policy.support_rows,
            );
            if positives.len() < minimum_rows {
                continue;
            }
            let support_watermark = positives
                .iter()
                .map(|frame| frame.observed_at_unix_nanos)
                .max()
                .unwrap_or(0);
            let negatives = dedupe_frames(
                pool.negatives
                    .iter()
                    .filter(|frame| {
                        frame.verifier_label != Some(true)
                            || !crate::synthesis::program_is_consistent(selected_program, frame)
                    })
                    .cloned()
                    .chain(
                        all_positives
                            .iter()
                            .filter(|frame| {
                                frame.observed_at_unix_nanos <= support_watermark
                                    && !crate::synthesis::program_is_consistent(
                                        selected_program,
                                        frame,
                                    )
                            })
                            .map(|frame| {
                                let mut negative = frame.clone();
                                negative.verifier_label = Some(false);
                                negative
                            }),
                    ),
            );
            let cohort_programs = programs
                .iter()
                .filter(|(_, program)| {
                    positives
                        .iter()
                        .all(|frame| crate::synthesis::program_is_consistent(program, frame))
                })
                .map(|(_, program)| program.clone())
                .collect::<Vec<_>>();
            let invariants = if subcenter.atom_ids.is_empty() {
                derive_cohort_invariants(&pool.invariants, &positives, &negatives)
            } else {
                derive_subcenter_invariants(&subcenter.atom_ids, &positives, &negatives)
            };
            let cohort_id_sha256 = crate::sha256_bytes(
                &serde_json::to_vec(&(
                    "nando.cegis-cohort.v9",
                    pool.teacher_signature_sha256.as_str(),
                    selected_digest.as_str(),
                    subcenter.atom_ids.as_slice(),
                    repair_watermark_unix_nanos,
                ))
                .unwrap_or_default(),
            );
            cohorts.push(ProgramCohort {
                cohort_id_sha256,
                positives,
                negatives,
                invariants,
                programs: cohort_programs,
            });
        }
        for index in selected_indices {
            uncovered.remove(index);
        }
    }
    cohorts
}

fn discover_clean_applicability_subcenters(
    program: &ResponseProgram,
    discovery_support: &[crate::RelationFrame],
    negatives: &[crate::RelationFrame],
    minimum_rows: usize,
    limit: usize,
) -> Vec<ApplicabilitySubcenter> {
    let positives = discovery_support
        .iter()
        .filter(|frame| crate::synthesis::program_is_consistent(program, frame))
        .collect::<Vec<_>>();
    if positives.len() < minimum_rows {
        return Vec::new();
    }
    let mut atom_evidence = BTreeMap::<u64, (usize, u64)>::new();
    for frame in &positives {
        for atom in relation_frame_online_routing_atom_ids(frame) {
            let evidence = atom_evidence.entry(atom).or_default();
            evidence.0 = evidence.0.saturating_add(1);
            evidence.1 = evidence.1.saturating_add(frame.estimated_input_tokens);
        }
    }
    let mut ranked_atoms = atom_evidence.into_iter().collect::<Vec<_>>();
    ranked_atoms.sort_by(|left, right| {
        right
            .1
            .0
            .cmp(&left.1.0)
            .then_with(|| right.1.1.cmp(&left.1.1))
            .then_with(|| left.0.cmp(&right.0))
    });
    let atoms = ranked_atoms
        .into_iter()
        .take(MAX_APPLICABILITY_ATOMS)
        .map(|(atom, _)| atom)
        .collect::<Vec<_>>();
    let mut predicates = atoms.iter().map(|atom| vec![*atom]).collect::<Vec<_>>();
    for (left_index, left) in atoms.iter().enumerate() {
        for right in atoms.iter().skip(left_index.saturating_add(1)) {
            predicates.push(vec![*left, *right]);
        }
    }
    let mut by_coverage = BTreeMap::<String, ApplicabilitySubcenter>::new();
    for atom_ids in predicates {
        let matching = positives
            .iter()
            .filter(|frame| frame_has_atoms(frame, &atom_ids))
            .copied()
            .collect::<Vec<_>>();
        if matching.len() < minimum_rows
            || matching.len() == positives.len()
            || negatives.iter().any(|frame| {
                frame_has_atoms(frame, &atom_ids)
                    && crate::synthesis::program_runtime_applicable(program, frame)
            })
        {
            continue;
        }
        let coverage_digest = crate::sha256_bytes(
            &serde_json::to_vec(
                &matching
                    .iter()
                    .map(|frame| frame.frame_id_sha256.as_str())
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_default(),
        );
        let candidate = ApplicabilitySubcenter {
            atom_ids,
            positive_rows: matching.len(),
            positive_tokens: matching
                .iter()
                .map(|frame| frame.estimated_input_tokens)
                .sum(),
        };
        by_coverage
            .entry(coverage_digest)
            .and_modify(|existing| {
                if candidate.atom_ids.len() < existing.atom_ids.len()
                    || (candidate.atom_ids.len() == existing.atom_ids.len()
                        && candidate.atom_ids < existing.atom_ids)
                {
                    *existing = candidate.clone();
                }
            })
            .or_insert(candidate);
    }
    let mut subcenters = by_coverage.into_values().collect::<Vec<_>>();
    subcenters.sort_by(|left, right| {
        right
            .positive_tokens
            .cmp(&left.positive_tokens)
            .then_with(|| right.positive_rows.cmp(&left.positive_rows))
            .then_with(|| left.atom_ids.len().cmp(&right.atom_ids.len()))
            .then_with(|| left.atom_ids.cmp(&right.atom_ids))
    });
    subcenters.truncate(limit);
    subcenters
}

fn derive_subcenter_invariants(
    required_atom_ids: &[u64],
    positives: &[crate::RelationFrame],
    negatives: &[crate::RelationFrame],
) -> Vec<RuntimeInvariant> {
    let mut candidates = BTreeMap::<Vec<u64>, RuntimeInvariant>::new();
    let positive_tokens = positives
        .iter()
        .map(|frame| frame.estimated_input_tokens)
        .sum::<u64>();
    insert_invariant_candidate(
        &mut candidates,
        required_atom_ids.to_vec(),
        positives.len(),
        positive_tokens,
        negatives,
    );
    for invariant in derive_cohort_invariants(&[], positives, negatives) {
        let mut combined = required_atom_ids.to_vec();
        combined.extend(invariant.atom_ids);
        insert_invariant_candidate(
            &mut candidates,
            combined,
            positives.len(),
            positive_tokens,
            negatives,
        );
    }
    let mut invariants = candidates.into_values().collect::<Vec<_>>();
    invariants.sort_by(|left, right| {
        left.negative_collisions
            .cmp(&right.negative_collisions)
            .then_with(|| left.atom_ids.len().cmp(&right.atom_ids.len()))
            .then_with(|| left.atom_ids.cmp(&right.atom_ids))
    });
    invariants.truncate(256);
    invariants
}

fn derive_cohort_invariants(
    inherited: &[RuntimeInvariant],
    positives: &[crate::RelationFrame],
    negatives: &[crate::RelationFrame],
) -> Vec<RuntimeInvariant> {
    let Some(first) = positives.first() else {
        return Vec::new();
    };
    let mut common = relation_frame_online_routing_atom_ids(first)
        .into_iter()
        .collect::<BTreeSet<_>>();
    for frame in &positives[1..] {
        let atoms = relation_frame_online_routing_atom_ids(frame)
            .into_iter()
            .collect::<BTreeSet<_>>();
        common.retain(|atom| atoms.contains(atom));
    }
    let positive_tokens = positives
        .iter()
        .map(|frame| frame.estimated_input_tokens)
        .sum::<u64>();
    let mut candidates = BTreeMap::<Vec<u64>, RuntimeInvariant>::new();
    for inherited in inherited {
        if inherited.atom_ids.iter().all(|atom| common.contains(atom)) {
            insert_invariant_candidate(
                &mut candidates,
                inherited.atom_ids.clone(),
                positives.len(),
                positive_tokens,
                negatives,
            );
        }
    }
    let common = common.into_iter().take(64).collect::<Vec<_>>();
    for atom in &common {
        insert_invariant_candidate(
            &mut candidates,
            vec![*atom],
            positives.len(),
            positive_tokens,
            negatives,
        );
    }
    'pairs: for (left_index, left) in common.iter().enumerate() {
        for right in common.iter().skip(left_index.saturating_add(1)) {
            insert_invariant_candidate(
                &mut candidates,
                vec![*left, *right],
                positives.len(),
                positive_tokens,
                negatives,
            );
            if candidates.len() >= 512 {
                break 'pairs;
            }
        }
    }
    let mut invariants = candidates.into_values().collect::<Vec<_>>();
    invariants.sort_by(|left, right| {
        left.negative_collisions
            .cmp(&right.negative_collisions)
            .then_with(|| left.atom_ids.len().cmp(&right.atom_ids.len()))
            .then_with(|| left.atom_ids.cmp(&right.atom_ids))
    });
    invariants.truncate(256);
    invariants
}

fn insert_invariant_candidate(
    candidates: &mut BTreeMap<Vec<u64>, RuntimeInvariant>,
    mut atom_ids: Vec<u64>,
    positive_rows: usize,
    positive_tokens: u64,
    negatives: &[crate::RelationFrame],
) {
    atom_ids.sort_unstable();
    atom_ids.dedup();
    if atom_ids.is_empty() || candidates.contains_key(&atom_ids) {
        return;
    }
    let negative_collisions = negatives
        .iter()
        .filter(|frame| frame_has_atoms(frame, &atom_ids))
        .count();
    candidates.insert(
        atom_ids.clone(),
        RuntimeInvariant {
            atom_ids,
            positive_rows: u32::try_from(positive_rows).unwrap_or(u32::MAX),
            positive_tokens,
            negative_collisions: u32::try_from(negative_collisions).unwrap_or(u32::MAX),
        },
    );
}

fn dedupe_frames<I>(frames: I) -> Vec<crate::RelationFrame>
where
    I: IntoIterator<Item = crate::RelationFrame>,
{
    let mut by_frame = BTreeMap::<String, crate::RelationFrame>::new();
    for frame in frames {
        by_frame
            .entry(frame.frame_id_sha256.clone())
            .or_insert(frame);
    }
    let mut frames = by_frame.into_values().collect::<Vec<_>>();
    frames.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });
    frames
}

fn run_state_slice(state: &mut CegisSearchState) -> usize {
    let budget = state.arena.exact_checks_per_slice();
    let invariants = candidate_invariants(&state.invariants, &state.positives);
    if invariants.is_empty() {
        state.blocker = Some("no_clean_pre_action_invariant".to_owned());
        return 0;
    }
    state.arena.begin_slice();
    let mut checks = 0_usize;
    while checks < budget {
        if state.active.is_none() {
            let Some(program) = state.arena.next_candidate() else {
                state.blocker = Some("version_space_exhausted".to_owned());
                break;
            };
            state.active = Some(ActiveEvaluation {
                program,
                positive_index: 0,
                negative_index: 0,
                invariant_index: 0,
                phase_delegated: false,
            });
        }
        let active = state.active.as_mut().expect("active CEGIS evaluation");
        let Some(required_atom_ids) = invariants
            .get(active.invariant_index)
            .map(|invariant| invariant.atom_ids.clone())
        else {
            state.arena.record_exact_check(
                active.program.node_id,
                false,
                "invariant_space_exhausted",
            );
            state.active = None;
            continue;
        };
        if let Some(frame) = state.positives.get(active.positive_index) {
            checks = checks.saturating_add(1);
            let consistent =
                crate::synthesis::program_is_consistent(&active.program.program, frame);
            state
                .arena
                .record_exact_check(active.program.node_id, consistent, "positive_mismatch");
            if consistent {
                active.positive_index = active.positive_index.saturating_add(1);
                continue;
            }
            let (kind, repair) = classify_program_counterexample(&active.program.program, frame);
            let repaired_programs = repaired_program_candidates(&active.program.program, frame);
            state.counterexamples.push(CegisCounterexample {
                frame_id_sha256: frame.frame_id_sha256.clone(),
                kind,
                repair,
                eliminated_program_sha256: Some(active.program.digest_sha256.clone()),
            });
            state.active = None;
            let inserted = state.arena.intern_all(repaired_programs);
            if inserted > 0 {
                state.generated_repair_programs = state
                    .generated_repair_programs
                    .saturating_add(u64::try_from(inserted).unwrap_or(u64::MAX));
                state.arena.rank_for_support(&state.positives);
            }
            continue;
        }
        if let Some(frame) = state.negatives.get(active.negative_index) {
            checks = checks.saturating_add(1);
            if frame.verifier_label == Some(true)
                && crate::synthesis::program_is_consistent(&active.program.program, frame)
            {
                state.arena.record_exact_check(
                    active.program.node_id,
                    true,
                    "cross_signature_transfer",
                );
                active.negative_index = active.negative_index.saturating_add(1);
                continue;
            }
            let false_accept = frame_has_atoms(frame, &required_atom_ids)
                && crate::synthesis::program_runtime_applicable(&active.program.program, frame);
            if false_accept {
                if active.phase_delegated {
                    state.arena.record_exact_check(
                        active.program.node_id,
                        true,
                        "phase_anti_center_required",
                    );
                    state.counterexamples.push(CegisCounterexample {
                        frame_id_sha256: frame.frame_id_sha256.clone(),
                        kind: CounterexampleKind::CompetingActionAccept,
                        repair: RepairAction::AddAntiCenter,
                        eliminated_program_sha256: None,
                    });
                    active.negative_index = active.negative_index.saturating_add(1);
                    continue;
                }
                let has_stronger_guard =
                    active.invariant_index.saturating_add(1) < invariants.len();
                state.arena.record_exact_check(
                    active.program.node_id,
                    true,
                    "negative_false_accept",
                );
                state.counterexamples.push(CegisCounterexample {
                    frame_id_sha256: frame.frame_id_sha256.clone(),
                    kind: CounterexampleKind::CompetingActionAccept,
                    repair: if has_stronger_guard {
                        RepairAction::StrengthenGuard
                    } else {
                        RepairAction::AddAntiCenter
                    },
                    eliminated_program_sha256: None,
                });
                if has_stronger_guard {
                    active.invariant_index = active.invariant_index.saturating_add(1);
                    active.positive_index = 0;
                    active.negative_index = 0;
                } else {
                    active.invariant_index = 0;
                    active.positive_index = state.positives.len();
                    active.negative_index = 0;
                    active.phase_delegated = true;
                }
                continue;
            }
            state
                .arena
                .record_exact_check(active.program.node_id, true, "negative_false_accept");
            active.negative_index = active.negative_index.saturating_add(1);
            continue;
        }
        let phase_rank = state
            .arena
            .phase_rank(active.program.node_id)
            .unwrap_or(u32::MAX);
        state.winner = Some(CegisWinner {
            cohort_id_sha256: state.cohort_id_sha256.clone(),
            teacher_signature_sha256: state.teacher_signature_sha256.clone(),
            action_symbol: state.action_symbol.clone(),
            program: active.program.program.clone(),
            required_atom_ids,
            positive_rows: state.positives.len(),
            negative_rows: state.negatives.len(),
            exact_checks: state.arena.report().exact_checks,
            search_slices: state.arena.report().slices_completed,
            phase_rank,
            support_frame_ids: state
                .positives
                .iter()
                .map(|frame| frame.frame_id_sha256.clone())
                .collect(),
            support_watermark_unix_nanos: state
                .positives
                .iter()
                .map(|frame| frame.observed_at_unix_nanos)
                .max()
                .unwrap_or(0),
            repair_watermark_unix_nanos: state.repair_watermark_unix_nanos,
        });
        state.active = None;
        break;
    }
    checks
}

fn repaired_program_candidates(
    eliminated: &ResponseProgram,
    frame: &crate::RelationFrame,
) -> Vec<ResponseProgram> {
    let kind = crate::response_program_kind(eliminated);
    let eliminated_digest =
        crate::sha256_bytes(&serde_json::to_vec(eliminated).unwrap_or_default());
    let mut repairs = BTreeMap::<String, ResponseProgram>::new();
    for program in
        crate::synthesis::enumerate_response_program_candidates(std::slice::from_ref(frame))
    {
        if crate::response_program_kind(&program) != kind
            || !crate::synthesis::program_is_consistent(&program, frame)
        {
            continue;
        }
        let digest = crate::sha256_bytes(&serde_json::to_vec(&program).unwrap_or_default());
        if digest != eliminated_digest {
            repairs.entry(digest).or_insert(program);
        }
    }
    repairs.into_values().collect()
}

fn candidate_invariants(
    invariants: &[RuntimeInvariant],
    positives: &[crate::RelationFrame],
) -> Vec<RuntimeInvariant> {
    let mut candidates = invariants
        .iter()
        .filter(|invariant| {
            positives
                .iter()
                .all(|frame| frame_has_atoms(frame, &invariant.atom_ids))
        })
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.negative_collisions
            .cmp(&right.negative_collisions)
            .then_with(|| left.atom_ids.len().cmp(&right.atom_ids.len()))
            .then_with(|| right.positive_tokens.cmp(&left.positive_tokens))
            .then_with(|| left.atom_ids.cmp(&right.atom_ids))
    });
    candidates
}

fn frame_has_atoms(frame: &crate::RelationFrame, required: &[u64]) -> bool {
    let observed = relation_frame_online_routing_atom_ids(frame);
    required
        .iter()
        .all(|atom| observed.binary_search(atom).is_ok())
}

fn classify_program_counterexample(
    program: &ResponseProgram,
    frame: &crate::RelationFrame,
) -> (CounterexampleKind, RepairAction) {
    let hypotheses = ground_roles(frame);
    if hypotheses.len() != 1 || hypotheses[0].competing_binding_count != 0 {
        return (
            CounterexampleKind::WrongRoleBinding,
            RepairAction::RegroundRoles,
        );
    }
    match program.operation {
        ResponseOperation::ProjectSelectedValue { .. }
        | ResponseOperation::ComposeCollection { .. } => (
            CounterexampleKind::OutputMismatch,
            RepairAction::RepairRenderer,
        ),
        ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. }
        | ResponseOperation::ProjectStatus { .. } => (
            CounterexampleKind::LayoutFailure,
            RepairAction::ReplaceSelector,
        ),
        _ => (
            CounterexampleKind::SameActionDifferentProgram,
            RepairAction::RejectHypothesis,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AtomSource, AtomValueType, RELATION_FRAME_SCHEMA, RelationAtom, ResponseValueSelector,
        SOURCE_NEUTRAL_EXTRACTOR_VERSION, teacher_program_signature,
    };

    fn function_frame(
        index: u64,
        event_time: u64,
        action: &str,
        capabilities: &[u64],
    ) -> crate::RelationFrame {
        let mut atoms = vec![
            RelationAtom::ToolKind {
                value: "exec".to_owned(),
            },
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 1,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Observation,
                value_sha256: "a".repeat(64),
            },
            RelationAtom::UniqueSlot { slot_id: 1 },
            RelationAtom::ObservationSelector {
                slot_id: 1,
                selector: ResponseValueSelector::UniqueScalar {
                    value_type: AtomValueType::Identifier,
                },
            },
            RelationAtom::TypedSlot {
                slot_id: 2,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Action,
                value_sha256: "a".repeat(64),
            },
            RelationAtom::SlotEquality {
                left_slot: 1,
                right_slot: 2,
            },
            RelationAtom::ActionFunction {
                value: action.to_owned(),
            },
            RelationAtom::ActionRoleArgument {
                name: "session_id".to_owned(),
                slot_id: 2,
                value_type: None,
            },
        ];
        atoms.extend(
            capabilities
                .iter()
                .map(|atom_id| RelationAtom::ClientCapabilityAtom { atom_id: *atom_id }),
        );
        crate::RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: format!("{index:064x}"),
            event_id_sha256: format!("{:064x}", index.saturating_add(10_000)),
            client_intent_id_sha256: format!("{:064x}", index.saturating_add(20_000)),
            session_id_sha256: format!("{:064x}", index.saturating_add(30_000)),
            observed_at_unix_nanos: event_time,
            estimated_input_tokens: 100,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms,
            evidence_ref_sha256: format!("{:064x}", index.saturating_add(40_000)),
        }
    }

    fn run_to_quiescence(cegis: &mut CegisCoordinator) {
        for _ in 0..1_024 {
            if !cegis.has_pending_work() {
                break;
            }
            let _ = cegis.run_next_slice();
        }
        assert!(!cegis.has_pending_work(), "CEGIS did not quiesce");
    }

    #[test]
    fn accepted_cross_signature_example_is_transfer_not_a_negative() {
        let positives = (0..32)
            .map(|row| function_frame(row + 1, row + 10, "wait", &[10]))
            .collect::<Vec<_>>();
        let signature = teacher_program_signature(&positives[0]).expect("teacher signature");
        let transfer = function_frame(1_000, 100, "wait", &[10]);
        let pool = TeacherPoolSnapshot {
            teacher_signature_sha256: signature,
            action_symbol: "function:wait".to_owned(),
            positives,
            negatives: vec![transfer],
            positive_rows: 32,
            negative_rows: 1,
            positive_tokens: 3_200,
            negative_tokens: 100,
            distinct_surfaces: 1,
            distinct_sessions: 32,
            invariants: Vec::new(),
        };
        let mut cegis = CegisCoordinator::new(VersionSpaceConfig::default(), 16);
        cegis.refresh_pool(&pool);
        run_to_quiescence(&mut cegis);

        assert_eq!(cegis.winners().len(), 1);
        assert_eq!(cegis.report().pools[0].counterexamples, 0);
    }

    #[test]
    fn clean_applicability_subset_gets_support_and_independent_future() {
        let positives = (0..96)
            .map(|row| {
                let layout = 100 + row % 4;
                let capabilities = if row < 16 || row >= 32 {
                    vec![10, 20, layout]
                } else {
                    vec![10, 30, layout]
                };
                function_frame(row + 1, row + 10, "wait", &capabilities)
            })
            .collect::<Vec<_>>();
        let signature = teacher_program_signature(&positives[0]).expect("teacher signature");
        let clean_atoms = relation_frame_online_routing_atom_ids(&positives[0]);
        let dirty_atoms = relation_frame_online_routing_atom_ids(&positives[16]);
        let clean_specific_atom = clean_atoms
            .iter()
            .find(|atom| dirty_atoms.binary_search(atom).is_err())
            .copied()
            .expect("clean applicability atom");
        let pool = TeacherPoolSnapshot {
            teacher_signature_sha256: signature,
            action_symbol: "function:wait".to_owned(),
            positives,
            negatives: vec![function_frame(1_000, 20, "write_stdin", &[10, 30, 100])],
            positive_rows: 96,
            negative_rows: 1,
            positive_tokens: 9_600,
            negative_tokens: 100,
            distinct_surfaces: 4,
            distinct_sessions: 96,
            invariants: Vec::new(),
        };
        let mut cegis = CegisCoordinator::new(VersionSpaceConfig::default(), 16);
        cegis.refresh_pool(&pool);
        run_to_quiescence(&mut cegis);

        let winner = cegis
            .winners()
            .into_iter()
            .find(|winner| winner.required_atom_ids.contains(&clean_specific_atom))
            .expect("clean applicability cohort winner");
        let future_eligible_ids = pool
            .positives
            .iter()
            .map(|frame| frame.frame_id_sha256.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let generation = crate::freeze_generation(
            &winner,
            &pool,
            crate::RolloverPolicy::default(),
            0,
            &future_eligible_ids,
        );
        assert_eq!(generation.support.len(), 32);
        assert!(generation.future.len() >= 32);
        assert_eq!(generation.wrong_future_rows, 0);
        assert_eq!(generation.blocker, None);
    }

    #[test]
    fn late_counterexample_participates_in_repair_subcenter_discovery() {
        let positives = (0..96)
            .map(|row| {
                let layout = 100 + row % 4;
                let capabilities = if row < 16 || row >= 32 {
                    vec![10, 20, layout]
                } else {
                    vec![10, 30, layout]
                };
                function_frame(row + 1, row + 10, "wait", &capabilities)
            })
            .collect::<Vec<_>>();
        let signature = teacher_program_signature(&positives[0]).expect("teacher signature");
        let clean_atoms = relation_frame_online_routing_atom_ids(&positives[0]);
        let dirty_atoms = relation_frame_online_routing_atom_ids(&positives[16]);
        let clean_specific_atom = clean_atoms
            .iter()
            .find(|atom| dirty_atoms.binary_search(atom).is_err())
            .copied()
            .expect("clean applicability atom");
        let mut pool = TeacherPoolSnapshot {
            teacher_signature_sha256: signature,
            action_symbol: "function:wait".to_owned(),
            positives,
            negatives: Vec::new(),
            positive_rows: 96,
            negative_rows: 0,
            positive_tokens: 9_600,
            negative_tokens: 0,
            distinct_surfaces: 4,
            distinct_sessions: 96,
            invariants: Vec::new(),
        };
        let mut cegis = CegisCoordinator::new(VersionSpaceConfig::default(), 16);
        cegis.refresh_pool(&pool);
        run_to_quiescence(&mut cegis);

        let counterexample = function_frame(1_000, 100, "write_stdin", &[10, 30, 100]);
        let transition = crate::teacher_transition_from_completed(&counterexample, None)
            .expect("teacher transition");
        assert_eq!(cegis.observe_global_transition(&transition), 1);
        pool.negatives.push(counterexample);
        pool.negative_rows = 1;
        pool.negative_tokens = 100;
        cegis.refresh_pool(&pool);
        run_to_quiescence(&mut cegis);

        assert!(
            cegis
                .winners()
                .iter()
                .any(|winner| winner.required_atom_ids.contains(&clean_specific_atom))
        );
    }

    #[test]
    fn post_freeze_indistinguishable_counterexample_blocks_repair() {
        let initial = (0..32)
            .map(|row| function_frame(row + 1, row + 10, "wait", &[10]))
            .collect::<Vec<_>>();
        let signature = teacher_program_signature(&initial[0]).expect("teacher signature");
        let mut pool = TeacherPoolSnapshot {
            teacher_signature_sha256: signature,
            action_symbol: "function:wait".to_owned(),
            positives: initial,
            negatives: vec![function_frame(1_000, 1, "write_stdin", &[])],
            positive_rows: 32,
            negative_rows: 1,
            positive_tokens: 3_200,
            negative_tokens: 100,
            distinct_surfaces: 1,
            distinct_sessions: 32,
            invariants: Vec::new(),
        };
        let mut cegis = CegisCoordinator::new(VersionSpaceConfig::default(), 16);
        cegis.refresh_pool(&pool);
        run_to_quiescence(&mut cegis);
        let first = cegis.winners();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].repair_watermark_unix_nanos, 0);

        let counterexample = function_frame(2_000, 100, "write_stdin", &[10]);
        let transition = crate::teacher_transition_from_completed(&counterexample, None)
            .expect("teacher transition");
        assert_eq!(cegis.observe_global_transition(&transition), 1);
        assert!(cegis.winners().is_empty());
        cegis.prepare_strategy_migration();
        pool.negatives.push(counterexample);
        cegis.refresh_pool(&pool);
        run_to_quiescence(&mut cegis);
        assert_eq!(cegis.report().repair_events, 1);
        assert_eq!(cegis.report().pools_waiting_after_repair, 1);
        assert!(cegis.winners().is_empty());
    }

    #[test]
    fn indistinguishable_training_negative_blocks_winner() {
        let positives = (0..32)
            .map(|row| function_frame(row + 1, row + 10, "wait", &[10]))
            .collect::<Vec<_>>();
        let signature = teacher_program_signature(&positives[0]).expect("teacher signature");
        let pool = TeacherPoolSnapshot {
            teacher_signature_sha256: signature,
            action_symbol: "function:wait".to_owned(),
            positives,
            negatives: vec![function_frame(1_000, 1, "write_stdin", &[10])],
            positive_rows: 32,
            negative_rows: 1,
            positive_tokens: 3_200,
            negative_tokens: 100,
            distinct_surfaces: 1,
            distinct_sessions: 32,
            invariants: Vec::new(),
        };
        let mut cegis = CegisCoordinator::new(VersionSpaceConfig::default(), 16);
        cegis.refresh_pool(&pool);
        run_to_quiescence(&mut cegis);

        assert!(cegis.winners().is_empty());
    }
}
