use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::rollover::support_partition_complete;
use crate::{
    CegisCoordinator, CegisReport, CegisWinner, CrossSurfaceFamilyDiscovery,
    FROZEN_PARTITION_VERSION, FamilyDiscoveryConfig, FamilyDiscoveryReport, FrozenGeneration,
    OpportunityBoard, OpportunityBoardReport, RolloverPolicy, TeacherPoolSnapshot,
    TeacherTransition, VersionSpaceConfig, freeze_generation, refresh_frozen_generation,
};

const TRANSFER_DISCOVERY_VERSION: u8 = 2;
const CROSS_POOL_NEGATIVE_REFRESH_INTERVAL: u64 = 64;
const MAX_PARITY_SIGNATURES: usize = 64;
const MAX_PARITY_CASES_PER_SIGNATURE: usize = 32;

fn parity_teacher_signature(frame: &crate::RelationFrame) -> String {
    crate::teacher_program_signature_from_action_atoms(&frame.atoms)
        .unwrap_or_else(|| "unknown_teacher_signature".to_owned())
}

fn cross_pool_negative_refresh_due(transitions_seen: u64) -> bool {
    transitions_seen > 0 && transitions_seen % CROSS_POOL_NEGATIVE_REFRESH_INTERVAL == 0
}

pub const SELF_TRAINING_STATE_SCHEMA_V2: &str = "nando.self-training-stream-state.v2";
pub const SELF_TRAINING_STATE_SCHEMA_V3: &str = "nando.self-training-stream-state.v3";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelfTrainingAdmissionCohort {
    pub winner: CegisWinner,
    pub generation: FrozenGeneration,
    pub pool: TeacherPoolSnapshot,
    pub runtime_parity_cases: Vec<crate::RuntimeParityCase>,
    pub semantic_alias_edges: Vec<crate::SemanticAliasEdge>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelfTrainingGenerationReport {
    pub partition_version: u32,
    pub cohort_id_sha256: String,
    pub teacher_signature_sha256: String,
    #[serde(default)]
    pub physical_adapter_count: usize,
    #[serde(default)]
    pub physical_adapter_signatures: Vec<String>,
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
    #[serde(default)]
    pub live_runtime_parity_rows: usize,
    #[serde(default)]
    pub after_future_watermark_rows: usize,
    #[serde(default)]
    pub independent_future_rows: usize,
    #[serde(default)]
    pub program_consistent_future_rows: usize,
    #[serde(default)]
    pub routed_future_rows: usize,
    pub runtime_parity_rows: usize,
    pub runtime_parity_tokens: u64,
    pub blocker: Option<String>,
}

#[derive(Clone, Copy, Debug, Default)]
struct FutureFilterDiagnostic {
    matching_rows: usize,
    matching_sessions: usize,
    post_repair_rows: usize,
    post_repair_sessions: usize,
    live_rows: usize,
    after_watermark_rows: usize,
    independent_rows: usize,
    consistent_rows: usize,
    routed_rows: usize,
}

#[derive(Clone, Debug)]
struct DerivedWinnerCohort {
    winner: CegisWinner,
    members: Vec<CegisWinner>,
    member_signatures: BTreeSet<String>,
    physical_adapter_count: usize,
}

fn rekey_parity_to_canonical_frames(
    cases: &mut BTreeMap<String, crate::RuntimeParityCase>,
    frames: &mut BTreeMap<String, crate::RelationFrame>,
    canonical_frames: &BTreeMap<(String, String, String), crate::RelationFrame>,
) {
    let old_cases = std::mem::take(cases);
    let old_frames = std::mem::take(frames);
    for (old_frame_id, mut parity) in old_cases {
        let Some(old_frame) = old_frames.get(&old_frame_id) else {
            cases.insert(old_frame_id, parity);
            continue;
        };
        let key = (
            old_frame.evidence_ref_sha256.clone(),
            old_frame.event_id_sha256.clone(),
            old_frame.session_id_sha256.clone(),
        );
        let canonical = canonical_frames.get(&key).unwrap_or(old_frame);
        let canonical_id = canonical.frame_id_sha256.clone();
        parity.evidence_ref_sha256.clone_from(&canonical_id);
        cases.insert(canonical_id.clone(), parity);
        frames.insert(canonical_id, canonical.clone());
    }
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
    #[serde(default)]
    pub runtime_parity_frames_total: usize,
    #[serde(default)]
    pub replay_support_parity_cases_total: usize,
    #[serde(default)]
    pub replay_support_parity_frames_total: usize,
    #[serde(default)]
    pub parity_discovery_key_overlap: usize,
    #[serde(default)]
    pub parity_accepted_frame_rows: usize,
    #[serde(default)]
    pub parity_signature_match_rows: usize,
    #[serde(default)]
    pub semantic_law_cohorts: usize,
    #[serde(default)]
    pub semantic_law_physical_adapters: usize,
    #[serde(default)]
    pub semantic_law_blockers: BTreeMap<String, usize>,
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
    #[serde(default)]
    transfer_discovery_version: u8,
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
    dirty_derived_signatures: BTreeSet<String>,
    #[serde(default)]
    runtime_parity_cases: BTreeMap<String, crate::RuntimeParityCase>,
    #[serde(default)]
    runtime_parity_frames: BTreeMap<String, crate::RelationFrame>,
    #[serde(default)]
    replay_support_parity_cases: BTreeMap<String, crate::RuntimeParityCase>,
    #[serde(default)]
    replay_support_parity_frames: BTreeMap<String, crate::RelationFrame>,
}

impl StreamingSelfTrainingState {
    #[must_use]
    pub fn new(now_unix: u64) -> Self {
        Self {
            schema: SELF_TRAINING_STATE_SCHEMA_V3.to_owned(),
            transfer_discovery_version: TRANSFER_DISCOVERY_VERSION,
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
            dirty_derived_signatures: BTreeSet::new(),
            runtime_parity_cases: BTreeMap::new(),
            runtime_parity_frames: BTreeMap::new(),
            replay_support_parity_cases: BTreeMap::new(),
            replay_support_parity_frames: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn replay_support_parity_cases_total(&self) -> usize {
        self.replay_support_parity_cases.len()
    }

    pub(crate) fn runtime_parity_cases_for_frames<'a>(
        &self,
        frames: impl IntoIterator<Item = &'a crate::RelationFrame>,
    ) -> Vec<crate::RuntimeParityCase> {
        let mut by_canonical_key = BTreeMap::new();
        for (frame_id, parity_frame) in self
            .runtime_parity_frames
            .iter()
            .chain(self.replay_support_parity_frames.iter())
        {
            let parity = self
                .runtime_parity_cases
                .get(frame_id)
                .or_else(|| self.replay_support_parity_cases.get(frame_id));
            if let Some(parity) = parity {
                by_canonical_key.insert(
                    (
                        parity_frame.evidence_ref_sha256.as_str(),
                        parity_frame.event_id_sha256.as_str(),
                        parity_frame.session_id_sha256.as_str(),
                    ),
                    parity,
                );
            }
        }
        let mut seen = BTreeSet::new();
        frames
            .into_iter()
            .filter_map(|frame| {
                let canonical_key = (
                    frame.evidence_ref_sha256.as_str(),
                    frame.event_id_sha256.as_str(),
                    frame.session_id_sha256.as_str(),
                );
                if !seen.insert(canonical_key) {
                    return None;
                }
                by_canonical_key.get(&canonical_key).map(|case| {
                    let mut case = (*case).clone();
                    // Admission binds a receipt to the exact frozen frame
                    // selected by the restored miner, not to an older
                    // equivalent training-frame identifier.
                    case.evidence_ref_sha256 = frame.frame_id_sha256.clone();
                    case
                })
            })
            .collect()
    }

    pub fn observe_migration_transition(
        &mut self,
        transition: &TeacherTransition,
    ) -> Result<(), String> {
        if let Some(mut parity_case) = transition.runtime_parity_case.clone() {
            let frame = transition.as_training_relation_frame();
            let frame_id = frame.frame_id_sha256.clone();
            parity_case.evidence_ref_sha256 = frame_id.clone();
            self.replay_support_parity_cases
                .insert(frame_id.clone(), parity_case);
            self.replay_support_parity_frames.insert(frame_id, frame);
            while self.replay_support_parity_cases.len() > 1_024 {
                let Some(oldest) = self.replay_support_parity_cases.keys().next().cloned() else {
                    break;
                };
                self.replay_support_parity_cases.remove(&oldest);
                self.replay_support_parity_frames.remove(&oldest);
            }
        }
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

    pub fn prepare_effect_law_migration(&mut self) {
        self.repair_parity_frames_from_discovery();
        for (frame_id, parity) in std::mem::take(&mut self.runtime_parity_cases) {
            self.replay_support_parity_cases
                .entry(frame_id)
                .or_insert(parity);
        }
        for (frame_id, frame) in std::mem::take(&mut self.runtime_parity_frames) {
            self.replay_support_parity_frames
                .entry(frame_id)
                .or_insert(frame);
        }
        self.enforce_parity_reservoir_limit();
        let retained = self
            .discovery
            .pool_snapshots()
            .into_iter()
            .flat_map(|pool| pool.positives)
            .collect::<Vec<_>>();
        let mut aliases = crate::SemanticAliasGraph::default();
        for frame in retained {
            let Ok(mut transition) = crate::teacher_transition_from_completed(&frame, None) else {
                continue;
            };
            transition.runtime_parity_case = self
                .replay_support_parity_cases
                .get(&frame.frame_id_sha256)
                .cloned();
            aliases.observe_transition(&transition);
        }
        *self.discovery.semantic_alias_graph_mut() = aliases;
        self.schema = SELF_TRAINING_STATE_SCHEMA_V3.to_owned();
        self.transfer_discovery_version = TRANSFER_DISCOVERY_VERSION;
        self.prepare_rebuild(false);
    }

    pub fn prepare_phase_route_migration(&mut self) {
        self.repair_parity_frames_from_discovery();
        for (frame_id, parity) in std::mem::take(&mut self.runtime_parity_cases) {
            self.replay_support_parity_cases
                .entry(frame_id)
                .or_insert(parity);
        }
        for (frame_id, frame) in std::mem::take(&mut self.runtime_parity_frames) {
            self.replay_support_parity_frames
                .entry(frame_id)
                .or_insert(frame);
        }
        self.enforce_parity_reservoir_limit();
        self.prepare_rebuild(false);
    }

    pub fn prepare_replay_seed(&mut self) {
        self.repair_parity_frames_from_discovery();
        self.enforce_parity_reservoir_limit();
        self.discovery
            .enforce_runtime_limits(FamilyDiscoveryConfig::default());
        self.discovery.rebuild_transfer_subcenters();
        self.transfer_discovery_version = TRANSFER_DISCOVERY_VERSION;
        self.prepare_rebuild(false);
    }

    pub fn prepare_incremental_replay_seed<I>(&mut self, signatures: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.repair_parity_frames_from_discovery();
        self.enforce_parity_reservoir_limit();
        self.discovery
            .enforce_runtime_limits(FamilyDiscoveryConfig::default());
        let mut queued = self.rebuild_queue.iter().cloned().collect::<BTreeSet<_>>();
        for signature in signatures {
            if queued.insert(signature.clone())
                && self.discovery.pool_snapshot(&signature).is_some()
            {
                self.rebuild_queue.push_back(signature.clone());
                self.dirty_derived_signatures.insert(signature);
            }
        }
    }

    fn prepare_rebuild(&mut self, clear_runtime_parity: bool) {
        self.cegis.prepare_strategy_migration();
        self.generations.clear();
        if clear_runtime_parity {
            self.runtime_parity_cases.clear();
            self.runtime_parity_frames.clear();
            self.replay_support_parity_cases.clear();
            self.replay_support_parity_frames.clear();
        }
        self.negative_refresh_cursor = None;
        self.rebuild_queue = self
            .discovery
            .pool_snapshots()
            .into_iter()
            .map(|pool| pool.teacher_signature_sha256)
            .collect();
        self.dirty_derived_signatures = self.rebuild_queue.iter().cloned().collect();
    }

    fn repair_parity_frames_from_discovery(&mut self) {
        let mut canonical_frames = BTreeMap::new();
        for pool in self.discovery.pool_snapshots() {
            for frame in pool.positives {
                canonical_frames.insert(
                    (
                        frame.evidence_ref_sha256.clone(),
                        frame.event_id_sha256.clone(),
                        frame.session_id_sha256.clone(),
                    ),
                    frame,
                );
            }
        }
        rekey_parity_to_canonical_frames(
            &mut self.runtime_parity_cases,
            &mut self.runtime_parity_frames,
            &canonical_frames,
        );
        rekey_parity_to_canonical_frames(
            &mut self.replay_support_parity_cases,
            &mut self.replay_support_parity_frames,
            &canonical_frames,
        );
    }

    pub fn prepare_teacher_signature_migration(&mut self) -> Result<(), String> {
        self.discovery
            .enforce_runtime_limits(FamilyDiscoveryConfig::default());

        // Historical parity material may still use the pre-canonical frame ID.
        // Rekey it before enriching action schemas, and include the bounded
        // replay receipts so migration does not discard independently verified
        // support merely because it predates the live parity reservoir.
        self.repair_parity_frames_from_discovery();
        self.enforce_parity_reservoir_limit();
        let mut parity_cases = self.replay_support_parity_cases.clone();
        parity_cases.extend(self.runtime_parity_cases.clone());
        self.discovery.enrich_action_schemas(&parity_cases);
        self.discovery.recanonicalize_teacher_signatures()?;
        self.repair_parity_frames_from_discovery();
        self.cegis.prepare_strategy_migration();
        self.generations.clear();
        self.negative_refresh_cursor = None;
        self.rebuild_queue = self
            .discovery
            .pool_snapshots()
            .into_iter()
            .map(|pool| pool.teacher_signature_sha256)
            .collect();
        self.dirty_derived_signatures = self.rebuild_queue.iter().cloned().collect();
        Ok(())
    }

    pub fn repair_missing_synthesis_state(&mut self) {
        self.repair_parity_frames_from_discovery();
        self.enforce_parity_reservoir_limit();
        if self.transfer_discovery_version < TRANSFER_DISCOVERY_VERSION {
            self.discovery.rebuild_transfer_subcenters();
            self.transfer_discovery_version = TRANSFER_DISCOVERY_VERSION;
            let mut queued = self.rebuild_queue.iter().cloned().collect::<BTreeSet<_>>();
            for pool in self.discovery.pool_snapshots() {
                if queued.insert(pool.teacher_signature_sha256.clone()) {
                    self.rebuild_queue
                        .push_back(pool.teacher_signature_sha256.clone());
                    self.dirty_derived_signatures
                        .insert(pool.teacher_signature_sha256);
                }
            }
        }
        self.opportunity.repair_teacher_aggregates();
        for pool in self.discovery.pool_snapshots() {
            for frame in pool.positives {
                if let Ok(transition) = crate::teacher_transition_from_completed(&frame, None) {
                    self.opportunity.reconcile_teacher_transition(&transition);
                }
            }
        }
        self.opportunity.repair_teacher_aggregates();
        for winner in self.cegis.winners() {
            if self
                .generations
                .get(&winner.cohort_id_sha256)
                .is_none_or(|generation| generation.partition_version < FROZEN_PARTITION_VERSION)
            {
                self.dirty_derived_signatures
                    .insert(winner.teacher_signature_sha256);
            }
        }
        self.dirty_derived_signatures.extend(
            self.generations
                .values()
                .filter(|generation| generation.partition_version < FROZEN_PARTITION_VERSION)
                .map(|generation| generation.teacher_signature_sha256.clone()),
        );
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
        self.dirty_derived_signatures = self.rebuild_queue.iter().cloned().collect();
    }

    pub fn observe_transition(&mut self, transition: &TeacherTransition) -> Result<(), String> {
        self.observe_runtime_parity_case(transition);
        if !self.discovery.observe_transition(transition)? {
            return Ok(());
        }
        self.transitions_seen = self.transitions_seen.saturating_add(1);
        self.opportunity.observe_transition(transition);
        let signature = transition.outcome.action.signature_sha256.clone();
        self.discovery
            .semantic_alias_graph_mut()
            .clear_candidate_blockers_for_member(&signature);
        self.dirty_derived_signatures.insert(signature.clone());
        let invalidated = self.cegis.observe_global_transition(transition);
        if invalidated > 0 {
            self.dirty_derived_signatures.extend(
                self.discovery
                    .pool_snapshots()
                    .into_iter()
                    .map(|pool| pool.teacher_signature_sha256),
            );
        }
        let preferred_support_ids = self.parity_support_ids();
        if let Some(pool) = self.pool_snapshot_with_parity(&signature) {
            let rows = pool.positive_rows;
            let refresh_due =
                !transition.outcome.verifier.accepted || rows == 16 || (rows > 16 && rows % 8 == 0);
            if refresh_due {
                self.cegis
                    .refresh_pool_with_preferred_support(&pool, &preferred_support_ids);
            }
        }
        if cross_pool_negative_refresh_due(self.transitions_seen)
            && let Some(next_signature) = self
                .cegis
                .next_winner_teacher_signature(self.negative_refresh_cursor.as_deref())
        {
            self.negative_refresh_cursor = Some(next_signature.clone());
            if next_signature != signature
                && let Some(pool) = self.pool_snapshot_with_parity(&next_signature)
            {
                self.cegis
                    .refresh_pool_with_preferred_support(&pool, &preferred_support_ids);
            }
        }
        let now_unix = transition.outcome.completed_at_unix_nanos / 1_000_000_000;
        self.opportunity.try_roll_window(now_unix);
        Ok(())
    }

    pub fn observe_runtime_parity_case(&mut self, transition: &TeacherTransition) {
        if let Some(mut parity_case) = transition.runtime_parity_case.clone() {
            let training_frame = transition.as_training_relation_frame();
            let training_frame_id = training_frame.frame_id_sha256.clone();
            parity_case.evidence_ref_sha256 = training_frame_id.clone();
            self.runtime_parity_cases
                .insert(training_frame_id.clone(), parity_case);
            self.runtime_parity_frames
                .insert(training_frame_id, training_frame);
            self.enforce_parity_reservoir_limit();
        }
    }

    /// Keeps a proof-sized reservoir per learned program instead of allowing
    /// frequent surfaces to consume one global payload-heavy cache.
    fn enforce_parity_reservoir_limit(&mut self) {
        self.runtime_parity_cases
            .retain(|frame_id, _| self.runtime_parity_frames.contains_key(frame_id));
        self.runtime_parity_frames
            .retain(|frame_id, _| self.runtime_parity_cases.contains_key(frame_id));
        self.replay_support_parity_cases.retain(|frame_id, _| {
            !self.runtime_parity_cases.contains_key(frame_id)
                && self.replay_support_parity_frames.contains_key(frame_id)
        });
        self.replay_support_parity_frames.retain(|frame_id, _| {
            !self.runtime_parity_cases.contains_key(frame_id)
                && self.replay_support_parity_cases.contains_key(frame_id)
        });

        let mut grouped = BTreeMap::<String, Vec<(bool, u64, String)>>::new();
        for (frame_id, frame) in &self.runtime_parity_frames {
            grouped
                .entry(parity_teacher_signature(frame))
                .or_default()
                .push((true, frame.observed_at_unix_nanos, frame_id.clone()));
        }
        for (frame_id, frame) in &self.replay_support_parity_frames {
            grouped
                .entry(parity_teacher_signature(frame))
                .or_default()
                .push((false, frame.observed_at_unix_nanos, frame_id.clone()));
        }

        let mut signatures = grouped.into_iter().collect::<Vec<_>>();
        for (_, rows) in &mut signatures {
            rows.sort_by(|left, right| {
                right
                    .0
                    .cmp(&left.0)
                    .then_with(|| right.1.cmp(&left.1))
                    .then_with(|| left.2.cmp(&right.2))
            });
        }
        signatures.sort_by(|left, right| {
            right
                .1
                .first()
                .map_or(0, |row| row.1)
                .cmp(&left.1.first().map_or(0, |row| row.1))
                .then_with(|| left.0.cmp(&right.0))
        });

        let retained = signatures
            .into_iter()
            .take(MAX_PARITY_SIGNATURES)
            .flat_map(|(_, rows)| {
                rows.into_iter()
                    .take(MAX_PARITY_CASES_PER_SIGNATURE)
                    .map(|(_, _, frame_id)| frame_id)
            })
            .collect::<BTreeSet<_>>();
        self.runtime_parity_cases
            .retain(|frame_id, _| retained.contains(frame_id));
        self.runtime_parity_frames
            .retain(|frame_id, _| retained.contains(frame_id));
        self.replay_support_parity_cases
            .retain(|frame_id, _| retained.contains(frame_id));
        self.replay_support_parity_frames
            .retain(|frame_id, _| retained.contains(frame_id));
    }

    /// Continues cold synthesis without requiring another event. The worker
    /// calls this only while queued work exists and always in bounded slices.
    pub fn run_work_slice(&mut self) -> usize {
        let preferred_support_ids = self.parity_support_ids();
        if let Some(signature) = self.rebuild_queue.pop_front()
            && let Some(pool) = self.pool_snapshot_with_parity(&signature)
        {
            self.cegis
                .refresh_pool_with_preferred_support(&pool, &preferred_support_ids);
        }
        let checks = self.cegis.run_next_slice();
        if checks > 0 {
            self.work_slices_completed = self.work_slices_completed.saturating_add(1);
            self.exact_checks_completed = self
                .exact_checks_completed
                .saturating_add(u64::try_from(checks).unwrap_or(u64::MAX));
        }
        if self.rebuild_queue.is_empty() {
            let synthesis_quiescent = !self.cegis.has_pending_work();
            let alias_updates = self.prove_candidate_alias_support(synthesis_quiescent);
            if alias_updates > 0 || synthesis_quiescent {
                self.refresh_dirty_derived_state(None);
            }
        }
        checks
    }

    pub fn run_work_slice_for_signatures(&mut self, signatures: &BTreeSet<String>) -> usize {
        if signatures.is_empty() {
            return 0;
        }
        let preferred_support_ids = self.parity_support_ids();
        if let Some(position) = self
            .rebuild_queue
            .iter()
            .position(|signature| signatures.contains(signature))
            && let Some(signature) = self.rebuild_queue.remove(position)
            && let Some(pool) = self.pool_snapshot_with_parity(&signature)
        {
            self.cegis
                .refresh_pool_with_preferred_support(&pool, &preferred_support_ids);
        }
        let checks = self.cegis.run_next_slice_for_teacher_signatures(signatures);
        if checks > 0 {
            self.work_slices_completed = self.work_slices_completed.saturating_add(1);
            self.exact_checks_completed = self
                .exact_checks_completed
                .saturating_add(u64::try_from(checks).unwrap_or(u64::MAX));
        }
        let selected_synthesis_pending = self
            .rebuild_queue
            .iter()
            .any(|signature| signatures.contains(signature))
            || self
                .cegis
                .has_pending_work_for_teacher_signatures(signatures);
        if !selected_synthesis_pending {
            let alias_updates = self.prove_candidate_alias_support(!self.cegis.has_pending_work());
            if alias_updates > 0 {
                self.refresh_dirty_derived_state(None);
            } else {
                self.refresh_dirty_derived_state(Some(signatures));
            }
        }
        checks
    }

    fn refresh_dirty_derived_state(&mut self, selected: Option<&BTreeSet<String>>) {
        for winner in self.cegis.winners() {
            if self
                .generations
                .get(&winner.cohort_id_sha256)
                .is_none_or(|generation| generation.partition_version < FROZEN_PARTITION_VERSION)
            {
                self.dirty_derived_signatures
                    .insert(winner.teacher_signature_sha256);
            }
        }
        self.dirty_derived_signatures.extend(
            self.generations
                .values()
                .filter(|generation| generation.partition_version < FROZEN_PARTITION_VERSION)
                .map(|generation| generation.teacher_signature_sha256.clone()),
        );
        let signatures = self
            .dirty_derived_signatures
            .iter()
            .filter(|signature| selected.is_none_or(|selected| selected.contains(*signature)))
            .cloned()
            .collect::<BTreeSet<_>>();
        if signatures.is_empty() {
            return;
        }
        self.refresh_generations_for_signatures(&signatures);
        self.refresh_opportunity_search_for_signatures(&signatures);
        for signature in signatures {
            self.dirty_derived_signatures.remove(&signature);
        }
    }

    fn parity_support_ids(&self) -> BTreeSet<String> {
        self.runtime_parity_cases
            .keys()
            .chain(self.replay_support_parity_cases.keys())
            .cloned()
            .collect()
    }

    fn prove_candidate_alias_support(&mut self, finalize_missing_winners: bool) -> usize {
        let candidates = self
            .discovery
            .semantic_alias_graph()
            .candidate_edges()
            .cloned()
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return 0;
        }
        let mut updates = 0_usize;
        let mut winners_by_signature = BTreeMap::<String, Vec<CegisWinner>>::new();
        for winner in self.cegis.winners() {
            winners_by_signature
                .entry(winner.teacher_signature_sha256.clone())
                .or_default()
                .push(winner);
        }
        for winners in winners_by_signature.values_mut() {
            winners.sort_by(|left, right| {
                right
                    .positive_rows
                    .cmp(&left.positive_rows)
                    .then_with(|| left.phase_rank.cmp(&right.phase_rank))
                    .then_with(|| left.cohort_id_sha256.cmp(&right.cohort_id_sha256))
            });
        }

        for edge in candidates {
            let Some(left) = winners_by_signature
                .get(&edge.left_teacher_signature_sha256)
                .and_then(|winners| winners.first())
                .cloned()
            else {
                if finalize_missing_winners
                    && self
                        .discovery
                        .semantic_alias_graph_mut()
                        .set_candidate_blocker(
                            &edge.edge_sha256,
                            "left_exact_winner_missing".to_owned(),
                        )
                        .is_ok()
                {
                    updates = updates.saturating_add(1);
                }
                continue;
            };
            let Some(right) = winners_by_signature
                .get(&edge.right_teacher_signature_sha256)
                .and_then(|winners| winners.first())
                .cloned()
            else {
                if finalize_missing_winners
                    && self
                        .discovery
                        .semantic_alias_graph_mut()
                        .set_candidate_blocker(
                            &edge.edge_sha256,
                            "right_exact_winner_missing".to_owned(),
                        )
                        .is_ok()
                {
                    updates = updates.saturating_add(1);
                }
                continue;
            };
            let members = [left, right];
            match self.build_semantic_law_cohort(&edge.effect_graph_sha256, &members) {
                Ok(derived) => {
                    let support_receipts = derived.winner.support_frame_ids.clone();
                    let parity_receipts = support_receipts
                        .iter()
                        .filter_map(|frame_id| {
                            self.runtime_parity_cases
                                .get(frame_id)
                                .or_else(|| self.replay_support_parity_cases.get(frame_id))
                                .map(|receipt| receipt.evidence_ref_sha256.clone())
                        })
                        .collect::<Vec<_>>();
                    let wave_proof_sha256 = crate::sha256_bytes(
                        &serde_json::to_vec(&(
                            "nando.semantic-alias-support-wave-ranking.v1",
                            edge.effect_graph_sha256.as_str(),
                            members
                                .iter()
                                .map(|winner| {
                                    (
                                        winner.cohort_id_sha256.as_str(),
                                        winner.phase_rank,
                                        winner.exact_checks,
                                        winner.search_slices,
                                    )
                                })
                                .collect::<Vec<_>>(),
                            &derived.winner.program,
                        ))
                        .unwrap_or_default(),
                    );
                    match self
                        .discovery
                        .semantic_alias_graph_mut()
                        .mark_support_proven(
                            &edge.edge_sha256,
                            support_receipts,
                            parity_receipts,
                            wave_proof_sha256,
                        ) {
                        Ok(()) => {
                            updates = updates.saturating_add(1);
                            self.dirty_derived_signatures
                                .insert(edge.left_teacher_signature_sha256.clone());
                            self.dirty_derived_signatures
                                .insert(edge.right_teacher_signature_sha256.clone());
                        }
                        Err(error) => {
                            if self
                                .discovery
                                .semantic_alias_graph_mut()
                                .set_candidate_blocker(
                                    &edge.edge_sha256,
                                    format!("support_proof:{error}"),
                                )
                                .is_ok()
                            {
                                updates = updates.saturating_add(1);
                            }
                        }
                    }
                }
                Err(blocker) => {
                    let terminal = blocker.contains("mismatch")
                        || blocker.contains("unseparable")
                        || blocker.contains("wrong");
                    if terminal {
                        if self
                            .discovery
                            .semantic_alias_graph_mut()
                            .reject(&edge.edge_sha256, blocker)
                            .is_ok()
                        {
                            updates = updates.saturating_add(1);
                        }
                    } else {
                        if self
                            .discovery
                            .semantic_alias_graph_mut()
                            .set_candidate_blocker(&edge.edge_sha256, blocker)
                            .is_ok()
                        {
                            updates = updates.saturating_add(1);
                        }
                    }
                }
            }
        }
        updates
    }

    fn derived_winner_cohorts(&self) -> Vec<DerivedWinnerCohort> {
        self.derived_winner_cohorts_with_blockers().0
    }

    fn derived_winner_cohorts_with_blockers(
        &self,
    ) -> (Vec<DerivedWinnerCohort>, BTreeMap<String, usize>) {
        let exact_winners = self.cegis.winners();
        let mut winner_by_signature = BTreeMap::<String, Vec<CegisWinner>>::new();
        for winner in exact_winners {
            winner_by_signature
                .entry(winner.teacher_signature_sha256.clone())
                .or_default()
                .push(winner);
        }
        let mut grouped_signatures = BTreeSet::new();
        let mut by_law = BTreeMap::<String, Vec<CegisWinner>>::new();
        for (law, signatures) in self.discovery.semantic_alias_graph().proven_components() {
            let mut members = Vec::new();
            for signature in signatures {
                if let Some(winners) = winner_by_signature.get(&signature) {
                    members.extend(winners.iter().cloned());
                    grouped_signatures.insert(signature);
                }
            }
            if !members.is_empty() {
                by_law.insert(law, members);
            }
        }
        for (signature, winners) in winner_by_signature {
            if !grouped_signatures.contains(&signature) {
                by_law.insert(signature, winners);
            }
        }
        let mut cohorts = Vec::new();
        let mut blockers = BTreeMap::<String, usize>::new();
        for (law, mut members) in by_law {
            members.sort_by(|left, right| {
                left.teacher_signature_sha256
                    .cmp(&right.teacher_signature_sha256)
                    .then_with(|| left.cohort_id_sha256.cmp(&right.cohort_id_sha256))
            });
            let distinct_signatures = members
                .iter()
                .map(|winner| winner.teacher_signature_sha256.as_str())
                .collect::<BTreeSet<_>>()
                .len();
            let derived =
                (distinct_signatures >= 2).then(|| self.build_semantic_law_cohort(&law, &members));
            match derived {
                Some(Ok(derived)) => cohorts.push(derived),
                Some(Err(blocker)) => {
                    *blockers.entry(blocker).or_default() += 1;
                    cohorts.extend(members.into_iter().map(|winner| DerivedWinnerCohort {
                        member_signatures: BTreeSet::from([
                            winner.teacher_signature_sha256.clone(),
                        ]),
                        physical_adapter_count: 1,
                        members: vec![winner.clone()],
                        winner,
                    }));
                }
                None => {
                    cohorts.extend(members.into_iter().map(|winner| DerivedWinnerCohort {
                        member_signatures: BTreeSet::from([
                            winner.teacher_signature_sha256.clone(),
                        ]),
                        physical_adapter_count: 1,
                        members: vec![winner.clone()],
                        winner,
                    }));
                }
            }
        }
        cohorts.sort_by(|left, right| {
            left.winner
                .cohort_id_sha256
                .cmp(&right.winner.cohort_id_sha256)
        });
        (cohorts, blockers)
    }

    fn build_semantic_law_cohort(
        &self,
        law_signature: &str,
        members: &[CegisWinner],
    ) -> Result<DerivedWinnerCohort, String> {
        let member_signatures = members
            .iter()
            .map(|winner| winner.teacher_signature_sha256.clone())
            .collect::<BTreeSet<_>>();
        if member_signatures.len() < 2 {
            return Err("semantic_law_exact_signatures_below_two".to_owned());
        }
        let mut program_classes =
            BTreeMap::<String, (crate::ResponseProgram, Vec<CegisWinner>)>::new();
        for winner in members {
            let digest = crate::sha256_bytes(
                &serde_json::to_vec(&winner.program)
                    .map_err(|error| format!("semantic_law_program_encode:{error}"))?,
            );
            program_classes
                .entry(digest)
                .or_insert_with(|| (winner.program.clone(), Vec::new()))
                .1
                .push(winner.clone());
        }
        let mut parity_rows_by_id = BTreeMap::new();
        for signature in &member_signatures {
            let Some(pool) = self.discovery.pool_snapshot(signature) else {
                continue;
            };
            for frame in pool.positives {
                let Some(case) = self
                    .runtime_parity_cases
                    .get(&frame.frame_id_sha256)
                    .or_else(|| self.replay_support_parity_cases.get(&frame.frame_id_sha256))
                    .cloned()
                else {
                    continue;
                };
                parity_rows_by_id.insert(
                    frame.frame_id_sha256.clone(),
                    (signature.clone(), frame, case),
                );
            }
        }
        let parity_rows = parity_rows_by_id.into_values().collect::<Vec<_>>();
        if parity_rows.len() < self.rollover_policy.support_rows {
            return Err(format!(
                "semantic_law_parity_rows_below_{}:{}",
                self.rollover_policy.support_rows,
                parity_rows.len()
            ));
        }
        let mut variants = Vec::new();
        let mut exact_only_variants = Vec::new();
        let mut adapter_training = Vec::new();
        let mut adapter_exact_rows = 0_usize;
        let mut adapter_wrong_rows = 0_usize;
        let mut adapter_abstain_rows = 0_usize;
        let mut adapter_same_name_rows = 0_usize;
        let mut adapter_same_arguments_rows = 0_usize;
        let mut adapter_same_name_and_arguments_rows = 0_usize;
        for (program, _class_members) in program_classes.values() {
            let mut positives = Vec::new();
            let mut negatives = Vec::new();
            for (_, _, case) in &parity_rows {
                let atoms =
                    crate::runtime::actor_adapter_phase_atom_ids(program, &case.provider_payload);
                if atoms.is_empty() {
                    continue;
                }
                let execution =
                    crate::execute_response(program, &case.request_text, &case.provider_payload);
                let positive = execution.status == crate::ResponseExecutionStatus::Executed
                    && execution.response.as_deref().is_some_and(|actual| {
                        actual == case.expected_response
                            || crate::online_admission::responses_match_after_execution_budget_normalization(
                                actual,
                                &case.expected_response,
                            )
                    });
                if positive {
                    adapter_exact_rows = adapter_exact_rows.saturating_add(1);
                    positives.push(atoms);
                } else {
                    if execution.status == crate::ResponseExecutionStatus::Executed {
                        adapter_wrong_rows = adapter_wrong_rows.saturating_add(1);
                        if let Some(actual) = execution.response.as_deref()
                            && let (Ok(actual), Ok(expected)) = (
                                serde_json::from_str::<serde_json::Value>(actual),
                                serde_json::from_str::<serde_json::Value>(&case.expected_response),
                            )
                        {
                            let same_name = actual.get("name") == expected.get("name");
                            let same_arguments =
                                actual.get("arguments") == expected.get("arguments");
                            adapter_same_name_rows =
                                adapter_same_name_rows.saturating_add(usize::from(same_name));
                            adapter_same_arguments_rows = adapter_same_arguments_rows
                                .saturating_add(usize::from(same_arguments));
                            adapter_same_name_and_arguments_rows =
                                adapter_same_name_and_arguments_rows
                                    .saturating_add(usize::from(same_name && same_arguments));
                        }
                    } else {
                        adapter_abstain_rows = adapter_abstain_rows.saturating_add(1);
                    }
                    negatives.push(atoms);
                }
            }
            if positives.is_empty() {
                continue;
            }
            let negative_fingerprints = negatives
                .iter()
                .map(|atoms| crate::online_collection::adapter_wave_atom_fingerprint(atoms))
                .collect::<BTreeSet<_>>();
            let clean_positives = positives
                .into_iter()
                .filter(|atoms| {
                    !negative_fingerprints.contains(
                        &crate::online_collection::adapter_wave_atom_fingerprint(atoms),
                    )
                })
                .collect::<Vec<_>>();
            if clean_positives.is_empty() {
                return Err("semantic_law_adapter_without_clean_positive_parity".to_owned());
            }
            let variant = crate::ResponseConsensusVariant {
                program: program.clone(),
                allowed_layout_sha256: Vec::new(),
                required_request_atom_ids: Vec::new(),
            };
            if negatives.is_empty() {
                exact_only_variants.push(variant.clone());
            }
            adapter_training.push((clean_positives, negatives));
            variants.push(variant);
        }
        if variants.is_empty() {
            return Err(format!(
                "semantic_law_without_runtime_proven_adapter:variants={}:cases={}:exact={adapter_exact_rows}:wrong={adapter_wrong_rows}:abstain={adapter_abstain_rows}:same_name={adapter_same_name_rows}:same_arguments={adapter_same_arguments_rows}:same_name_and_arguments={adapter_same_name_and_arguments_rows}",
                program_classes.len(),
                parity_rows.len(),
            ));
        }
        let exact_only_program = if exact_only_variants.is_empty() {
            None
        } else if exact_only_variants.len() == 1 {
            exact_only_variants
                .first()
                .map(|variant| variant.program.clone())
        } else {
            Some(crate::ResponseProgram::unique_consensus(
                exact_only_variants.clone(),
            ))
        };
        let exact_only_program = exact_only_program.filter(|candidate| {
            let mut accepted = 0_usize;
            for (exact_signature, frame, parity) in &parity_rows {
                let covered = members.iter().any(|winner| {
                    exact_signature == &winner.teacher_signature_sha256
                        && crate::synthesis::program_is_consistent(&winner.program, frame)
                        && crate::cegis::winner_routes_frame(winner, frame)
                });
                if !covered {
                    continue;
                }
                let execution = crate::execute_response(
                    candidate,
                    &parity.request_text,
                    &parity.provider_payload,
                );
                if execution.status != crate::ResponseExecutionStatus::Executed {
                    continue;
                }
                let matches = execution.response.as_deref().is_some_and(|actual| {
                    actual == parity.expected_response
                        || crate::online_admission::responses_match_after_execution_budget_normalization(
                            actual,
                            &parity.expected_response,
                        )
                });
                if !matches {
                    return false;
                }
                accepted = accepted.saturating_add(1);
            }
            accepted >= self.rollover_policy.support_rows
        });
        let mut physical_adapter_count = variants.len();
        let program = if let Some(program) = exact_only_program {
            physical_adapter_count = exact_only_variants.len();
            program
        } else if variants.len() == 1 {
            variants
                .pop()
                .ok_or_else(|| "semantic_law_variant_missing".to_owned())?
                .program
        } else {
            let exact_consensus = crate::ResponseProgram::unique_consensus(variants.clone());
            let exact_resolves_all = parity_rows.iter().all(|(exact_signature, frame, parity)| {
                let covered = members.iter().any(|winner| {
                    exact_signature == &winner.teacher_signature_sha256
                        && crate::synthesis::program_is_consistent(&winner.program, frame)
                        && crate::cegis::winner_routes_frame(winner, frame)
                });
                if !covered {
                    return true;
                }
                let execution = crate::execute_response(
                    &exact_consensus,
                    &parity.request_text,
                    &parity.provider_payload,
                );
                execution.status == crate::ResponseExecutionStatus::Executed
                    && execution.response.as_deref().is_some_and(|actual| {
                        actual == parity.expected_response
                            || crate::online_admission::responses_match_after_execution_budget_normalization(
                                actual,
                                &parity.expected_response,
                            )
                    })
            });
            if exact_resolves_all {
                exact_consensus
            } else {
                let routes = adapter_training
                    .iter()
                    .map(|(positives, negatives)| {
                        crate::online_collection::fit_adapter_wave_route(positives, negatives, 16)
                            .ok_or_else(|| {
                                format!(
                                    "semantic_law_adapter_wave_unseparable:positives={}:negatives={}",
                                    positives.len(),
                                    negatives.len()
                                )
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                exact_consensus.with_adapter_wave(crate::ResponseAdapterWaveConsensus {
                    exact_budget: u16::try_from(routes.len().min(16))
                        .map_err(|error| format!("semantic_law_exact_budget:{error}"))?,
                    routes,
                })
            }
        };
        program
            .validate()
            .map_err(|error| format!("semantic_law_program_invalid:{error}"))?;
        let mut accepted_parity_rows = 0_usize;
        for (exact_signature, frame, parity) in &parity_rows {
            let covered = members.iter().any(|winner| {
                exact_signature == &winner.teacher_signature_sha256
                    && crate::synthesis::program_is_consistent(&winner.program, frame)
                    && crate::cegis::winner_routes_frame(winner, frame)
            });
            if !covered {
                continue;
            }
            let execution =
                crate::execute_response(&program, &parity.request_text, &parity.provider_payload);
            if execution.status != crate::ResponseExecutionStatus::Executed {
                continue;
            }
            let response_matches = execution.response.as_deref().is_some_and(|actual| {
                actual == parity.expected_response
                    || crate::online_admission::responses_match_after_execution_budget_normalization(
                        actual,
                        &parity.expected_response,
                    )
            });
            if !response_matches {
                return Err(format!(
                    "semantic_law_parity_mismatch:{}:{}",
                    frame.frame_id_sha256, execution.reason
                ));
            }
            accepted_parity_rows = accepted_parity_rows.saturating_add(1);
        }
        if accepted_parity_rows < self.rollover_policy.support_rows {
            return Err(format!(
                "semantic_law_clean_parity_rows_below_{}:{}",
                self.rollover_policy.support_rows, accepted_parity_rows
            ));
        }
        let mut required_atom_ids = members
            .first()
            .map(|winner| winner.required_atom_ids.clone())
            .unwrap_or_default();
        required_atom_ids.retain(|atom| {
            members
                .iter()
                .skip(1)
                .all(|winner| winner.required_atom_ids.binary_search(atom).is_ok())
        });
        let support_frame_ids = members
            .iter()
            .flat_map(|winner| winner.support_frame_ids.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let member_ids = members
            .iter()
            .map(|winner| winner.cohort_id_sha256.as_str())
            .collect::<Vec<_>>();
        let cohort_id_sha256 = crate::sha256_bytes(
            &serde_json::to_vec(&(
                "nando.semantic-law-cohort.v1",
                law_signature,
                member_ids,
                &program,
            ))
            .map_err(|error| format!("semantic_law_cohort_encode:{error}"))?,
        );
        let action_symbols = members
            .iter()
            .map(|winner| winner.action_symbol.as_str())
            .collect::<BTreeSet<_>>();
        let action_symbol = if action_symbols.len() == 1 {
            action_symbols
                .iter()
                .next()
                .ok_or_else(|| "semantic_law_action_symbol_missing".to_owned())?
                .to_string()
        } else {
            format!(
                "semantic_law:{}",
                &law_signature[..law_signature.len().min(12)]
            )
        };
        let winner = CegisWinner {
            cohort_id_sha256,
            teacher_signature_sha256: law_signature.to_owned(),
            action_symbol,
            program,
            required_atom_ids,
            anti_center_atom_sets: Vec::new(),
            learned_wave_route: None,
            positive_rows: members.iter().map(|winner| winner.positive_rows).sum(),
            negative_rows: members.iter().map(|winner| winner.negative_rows).sum(),
            exact_checks: members.iter().map(|winner| winner.exact_checks).sum(),
            search_slices: members.iter().map(|winner| winner.search_slices).sum(),
            phase_rank: members
                .iter()
                .map(|winner| winner.phase_rank)
                .min()
                .unwrap_or(0),
            support_frame_ids,
            support_watermark_unix_nanos: members
                .iter()
                .map(|winner| winner.support_watermark_unix_nanos)
                .max()
                .unwrap_or(0),
            repair_watermark_unix_nanos: members
                .iter()
                .map(|winner| winner.repair_watermark_unix_nanos)
                .max()
                .unwrap_or(0),
        };
        Ok(DerivedWinnerCohort {
            winner,
            members: members.to_vec(),
            member_signatures,
            physical_adapter_count,
        })
    }

    fn pool_snapshot_with_parity(&self, signature: &str) -> Option<TeacherPoolSnapshot> {
        let mut pool = self.discovery.pool_snapshot(signature)?;
        let mut positives = pool
            .positives
            .into_iter()
            .map(|frame| (frame.frame_id_sha256.clone(), frame))
            .collect::<BTreeMap<_, _>>();
        for frame in self
            .runtime_parity_frames
            .values()
            .chain(self.replay_support_parity_frames.values())
        {
            if frame.verifier_label == Some(true)
                && crate::teacher_program_signature(frame).as_deref() == Some(signature)
            {
                positives
                    .entry(frame.frame_id_sha256.clone())
                    .or_insert_with(|| frame.clone());
            }
        }
        pool.positives = positives.into_values().collect();
        pool.positives.sort_by(|left, right| {
            left.observed_at_unix_nanos
                .cmp(&right.observed_at_unix_nanos)
                .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
        });
        pool.positive_rows = u64::try_from(pool.positives.len()).unwrap_or(u64::MAX);
        pool.positive_tokens = pool
            .positives
            .iter()
            .map(|frame| frame.estimated_input_tokens)
            .sum();
        pool.distinct_sessions = pool
            .positives
            .iter()
            .map(|frame| frame.session_id_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        pool.distinct_surfaces = pool
            .positives
            .iter()
            .filter_map(crate::relation_frame_structural_family_id)
            .collect::<BTreeSet<_>>()
            .len();
        Some(pool)
    }

    #[must_use]
    pub fn has_pending_work(&self) -> bool {
        !self.rebuild_queue.is_empty()
            || self.cegis.has_pending_work()
            || !self.dirty_derived_signatures.is_empty()
            || self
                .generations
                .values()
                .any(|generation| generation.partition_version < FROZEN_PARTITION_VERSION)
    }

    #[must_use]
    pub fn has_pending_work_for_signatures(&self, signatures: &BTreeSet<String>) -> bool {
        self.rebuild_queue
            .iter()
            .any(|signature| signatures.contains(signature))
            || self
                .cegis
                .has_pending_work_for_teacher_signatures(signatures)
            || self
                .dirty_derived_signatures
                .iter()
                .any(|signature| signatures.contains(signature))
            || self.generations.values().any(|generation| {
                generation.partition_version < FROZEN_PARTITION_VERSION
                    && signatures.contains(&generation.teacher_signature_sha256)
            })
    }

    #[must_use]
    pub fn admission_cohorts(&self) -> Vec<SelfTrainingAdmissionCohort> {
        if !self.opportunity.authority_safe() {
            return Vec::new();
        }
        let winners = self
            .derived_winner_cohorts()
            .into_iter()
            .map(|cohort| (cohort.winner.cohort_id_sha256.clone(), cohort))
            .collect::<BTreeMap<_, _>>();
        let mut cohorts = self
            .generations
            .iter()
            .filter(|(_, generation)| generation.blocker.is_none())
            .filter_map(|(cohort_id, generation)| {
                let derived = winners.get(cohort_id)?;
                let winner = derived.winner.clone();
                let pool = self.cohort_pool_snapshot(derived)?;
                Some(SelfTrainingAdmissionCohort {
                    winner,
                    pool,
                    generation: generation.clone(),
                    semantic_alias_edges: self
                        .discovery
                        .semantic_alias_graph()
                        .proven_edges_for_members(&derived.member_signatures),
                    runtime_parity_cases: generation
                        .support
                        .iter()
                        .chain(generation.future.iter())
                        .filter_map(|frame| {
                            self.runtime_parity_cases
                                .get(&frame.frame_id_sha256)
                                .or_else(|| {
                                    self.replay_support_parity_cases.get(&frame.frame_id_sha256)
                                })
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
        let (winner_cohorts, semantic_law_blockers) = self.derived_winner_cohorts_with_blockers();
        let winners = winner_cohorts
            .into_iter()
            .map(|cohort| (cohort.winner.cohort_id_sha256.clone(), cohort))
            .collect::<BTreeMap<_, _>>();
        let parity_diagnostics = winners
            .iter()
            .filter_map(|(cohort_id, derived)| {
                let winner = &derived.winner;
                let generation = self.generations.get(cohort_id)?;
                let pool = self.cohort_pool_snapshot(derived)?;
                let matching = pool
                    .positives
                    .iter()
                    .filter(|frame| {
                        (self
                            .runtime_parity_cases
                            .contains_key(&frame.frame_id_sha256)
                            || self
                                .replay_support_parity_cases
                                .contains_key(&frame.frame_id_sha256))
                            && crate::synthesis::program_is_consistent(&winner.program, frame)
                            && crate::cegis::winner_routes_frame(winner, frame)
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
                let support_ids = generation
                    .support
                    .iter()
                    .map(|frame| frame.frame_id_sha256.as_str())
                    .collect::<BTreeSet<_>>();
                let support_sessions = generation
                    .support
                    .iter()
                    .map(|frame| frame.session_id_sha256.as_str())
                    .collect::<BTreeSet<_>>();
                let support_intents = generation
                    .support
                    .iter()
                    .map(|frame| frame.client_intent_id_sha256.as_str())
                    .collect::<BTreeSet<_>>();
                let support_events = generation
                    .support
                    .iter()
                    .map(|frame| frame.event_id_sha256.as_str())
                    .collect::<BTreeSet<_>>();
                let future_watermark = generation
                    .support_watermark_unix_nanos
                    .max(winner.repair_watermark_unix_nanos);
                let live = pool
                    .positives
                    .iter()
                    .filter(|frame| {
                        self.runtime_parity_cases
                            .contains_key(&frame.frame_id_sha256)
                    })
                    .collect::<Vec<_>>();
                let after_watermark = live
                    .iter()
                    .filter(|frame| frame.observed_at_unix_nanos > future_watermark)
                    .copied()
                    .collect::<Vec<_>>();
                let independent = after_watermark
                    .iter()
                    .filter(|frame| {
                        !support_ids.contains(frame.frame_id_sha256.as_str())
                            && !winner.support_frame_ids.contains(&frame.frame_id_sha256)
                            && !support_sessions.contains(frame.session_id_sha256.as_str())
                            && !support_intents.contains(frame.client_intent_id_sha256.as_str())
                            && !support_events.contains(frame.event_id_sha256.as_str())
                    })
                    .copied()
                    .collect::<Vec<_>>();
                let consistent = independent
                    .iter()
                    .filter(|frame| crate::synthesis::program_is_consistent(&winner.program, frame))
                    .copied()
                    .collect::<Vec<_>>();
                let routed_rows = consistent
                    .iter()
                    .filter(|frame| crate::cegis::winner_routes_frame(winner, frame))
                    .count();
                Some((
                    cohort_id.clone(),
                    FutureFilterDiagnostic {
                        matching_rows: matching.len(),
                        matching_sessions,
                        post_repair_rows: post_repair.len(),
                        post_repair_sessions,
                        live_rows: live.len(),
                        after_watermark_rows: after_watermark.len(),
                        independent_rows: independent.len(),
                        consistent_rows: consistent.len(),
                        routed_rows,
                    },
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
                    physical_adapter_count: winners
                        .get(&generation.cohort_id_sha256)
                        .map_or(0, |cohort| cohort.physical_adapter_count),
                    physical_adapter_signatures: winners
                        .get(&generation.cohort_id_sha256)
                        .map(|cohort| cohort.member_signatures.iter().cloned().collect())
                        .unwrap_or_default(),
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
                                || self
                                    .replay_support_parity_cases
                                    .contains_key(&frame.frame_id_sha256)
                        })
                        .count(),
                    support_runtime_parity_tokens: generation
                        .support
                        .iter()
                        .filter(|frame| {
                            self.runtime_parity_cases
                                .contains_key(&frame.frame_id_sha256)
                                || self
                                    .replay_support_parity_cases
                                    .contains_key(&frame.frame_id_sha256)
                        })
                        .map(|frame| frame.estimated_input_tokens)
                        .sum(),
                    matching_runtime_parity_rows: parity.matching_rows,
                    matching_runtime_parity_sessions: parity.matching_sessions,
                    post_repair_runtime_parity_rows: parity.post_repair_rows,
                    post_repair_runtime_parity_sessions: parity.post_repair_sessions,
                    live_runtime_parity_rows: parity.live_rows,
                    after_future_watermark_rows: parity.after_watermark_rows,
                    independent_future_rows: parity.independent_rows,
                    program_consistent_future_rows: parity.consistent_rows,
                    routed_future_rows: parity.routed_rows,
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
        let winner_ids = winners.keys().cloned().collect::<BTreeSet<_>>();
        let semantic_law_cohorts = winners
            .values()
            .filter(|cohort| cohort.member_signatures.len() > 1)
            .count();
        let semantic_law_physical_adapters = winners
            .values()
            .filter(|cohort| cohort.member_signatures.len() > 1)
            .map(|cohort| cohort.physical_adapter_count)
            .sum();
        let admission_ready_cohorts = generations
            .iter()
            .filter(|generation| {
                generation.blocker.is_none() && winner_ids.contains(&generation.cohort_id_sha256)
            })
            .count();
        let opportunity = self.opportunity.report(now_unix);
        let discovery_frames = self
            .discovery
            .pool_snapshots()
            .into_iter()
            .flat_map(|pool| pool.positives)
            .map(|frame| (frame.frame_id_sha256.clone(), frame))
            .collect::<BTreeMap<_, _>>();
        let parity_frames = self
            .runtime_parity_frames
            .iter()
            .chain(self.replay_support_parity_frames.iter())
            .collect::<BTreeMap<_, _>>();
        let parity_discovery_key_overlap = parity_frames
            .keys()
            .filter(|frame_id| discovery_frames.contains_key(**frame_id))
            .count();
        let parity_accepted_frame_rows = parity_frames
            .values()
            .filter(|frame| frame.verifier_label == Some(true))
            .count();
        let parity_signature_match_rows = parity_frames
            .iter()
            .filter(|(frame_id, frame)| {
                discovery_frames.get(**frame_id).is_some_and(|discovery| {
                    crate::teacher_program_signature(frame)
                        == crate::teacher_program_signature(discovery)
                })
            })
            .count();
        let signal_tree = build_signal_tree(
            self.transitions_seen,
            &discovery,
            &cegis,
            &generations,
            admission_ready_cohorts,
        );
        SelfTrainingStateReport {
            schema: SELF_TRAINING_STATE_SCHEMA_V3.to_owned(),
            transitions_seen: self.transitions_seen,
            work_slices_completed: self.work_slices_completed,
            exact_checks_completed: self.exact_checks_completed,
            runtime_parity_cases_total: self.runtime_parity_cases.len(),
            runtime_parity_frames_total: self.runtime_parity_frames.len(),
            replay_support_parity_cases_total: self.replay_support_parity_cases.len(),
            replay_support_parity_frames_total: self.replay_support_parity_frames.len(),
            parity_discovery_key_overlap,
            parity_accepted_frame_rows,
            parity_signature_match_rows,
            semantic_law_cohorts,
            semantic_law_physical_adapters,
            semantic_law_blockers,
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

    pub(crate) fn bounded_teacher_frames_for_wave_migration(&self) -> Vec<crate::RelationFrame> {
        let mut unique = BTreeMap::<String, crate::RelationFrame>::new();
        for pool in self.discovery.pool_snapshots() {
            for frame in pool.positives.into_iter().chain(pool.negatives) {
                unique.entry(frame.frame_id_sha256.clone()).or_insert(frame);
            }
        }
        let mut frames = unique.into_values().collect::<Vec<_>>();
        frames.sort_by(|left, right| {
            left.observed_at_unix_nanos
                .cmp(&right.observed_at_unix_nanos)
                .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
        });
        frames
    }

    #[must_use]
    pub fn admission_ready_cohort_count(&self) -> usize {
        self.admission_cohorts().len()
    }

    fn refresh_generations_for_signatures(&mut self, signatures: &BTreeSet<String>) {
        self.refresh_generations_filtered(Some(signatures));
    }

    fn refresh_generations_filtered(&mut self, signatures: Option<&BTreeSet<String>>) {
        let winners = self
            .derived_winner_cohorts()
            .into_iter()
            .filter(|cohort| {
                signatures.is_none_or(|selected| {
                    cohort
                        .member_signatures
                        .iter()
                        .any(|signature| selected.contains(signature))
                })
            })
            .collect::<Vec<_>>();
        let support_eligible_ids = self
            .runtime_parity_cases
            .keys()
            .chain(self.replay_support_parity_cases.keys())
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let future_eligible_ids = self
            .runtime_parity_cases
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let live_ids = winners
            .iter()
            .map(|cohort| cohort.winner.cohort_id_sha256.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        self.generations
            .retain(|cohort_id, _| live_ids.contains(cohort_id.as_str()));
        for derived in winners {
            let winner = &derived.winner;
            let Some(pool) = self.cohort_pool_snapshot(&derived) else {
                continue;
            };
            let generation_number = self
                .generations
                .get(&winner.cohort_id_sha256)
                .map_or(0, |generation| generation.generation);
            let (next, allow_support_repartition) = if let Some(current) =
                self.generations.get(&winner.cohort_id_sha256)
            {
                let partition_upgrade = current.partition_version < FROZEN_PARTITION_VERSION;
                let incomplete_support =
                    !support_partition_complete(current.support.len(), self.rollover_policy);
                let incomplete_support_evidence = !generation_support_parity_complete(
                    current,
                    &support_eligible_ids,
                    self.rollover_policy,
                );
                let can_repartition =
                    partition_upgrade || incomplete_support || incomplete_support_evidence;
                let refrozen_generation = if incomplete_support_evidence {
                    generation_number.saturating_add(1)
                } else {
                    generation_number
                };
                let refrozen = can_repartition.then(|| {
                    freeze_generation(
                        winner,
                        &pool,
                        self.rollover_policy,
                        refrozen_generation,
                        &support_eligible_ids,
                        &future_eligible_ids,
                    )
                });
                let proof_repartition_improves = incomplete_support_evidence
                    && refrozen.as_ref().is_some_and(|candidate| {
                        support_partition_complete(candidate.support.len(), self.rollover_policy)
                            && candidate.support != current.support
                    });
                let repartition_improves = partition_upgrade
                    || incomplete_support
                    || proof_repartition_improves
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
                            winner,
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
                        winner,
                        &pool,
                        self.rollover_policy,
                        generation_number,
                        &support_eligible_ids,
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

    fn cohort_pool_snapshot(&self, cohort: &DerivedWinnerCohort) -> Option<TeacherPoolSnapshot> {
        let winner = &cohort.winner;
        let mut positives = BTreeMap::<String, crate::RelationFrame>::new();
        let mut negative_frames = BTreeMap::<String, crate::RelationFrame>::new();
        let mut invariants = Vec::new();
        for signature in &cohort.member_signatures {
            let pool = self.pool_snapshot_with_parity(signature)?;
            invariants.extend(pool.invariants);
            for frame in pool.positives {
                let signature = crate::teacher_program_signature(&frame);
                let routed = cohort.members.iter().any(|member| {
                    signature.as_deref() == Some(&member.teacher_signature_sha256)
                        && crate::synthesis::program_is_consistent(&member.program, &frame)
                        && crate::cegis::winner_routes_frame(member, &frame)
                });
                let parity_matches = self
                    .runtime_parity_cases
                    .get(&frame.frame_id_sha256)
                    .or_else(|| self.replay_support_parity_cases.get(&frame.frame_id_sha256))
                    .is_some_and(|parity| {
                        let execution = crate::execute_response(
                            &winner.program,
                            &parity.request_text,
                            &parity.provider_payload,
                        );
                        execution.status == crate::ResponseExecutionStatus::Executed
                            && execution.response.as_deref().is_some_and(|actual| {
                                actual == parity.expected_response
                                    || crate::online_admission::responses_match_after_execution_budget_normalization(
                                        actual,
                                        &parity.expected_response,
                                    )
                            })
                    });
                if routed && parity_matches {
                    positives
                        .entry(frame.frame_id_sha256.clone())
                        .or_insert(frame);
                } else {
                    let mut frame = frame;
                    frame.verifier_label = Some(false);
                    negative_frames
                        .entry(frame.frame_id_sha256.clone())
                        .or_insert(frame);
                }
            }
            for mut frame in pool.negatives {
                frame.verifier_label = Some(false);
                negative_frames
                    .entry(frame.frame_id_sha256.clone())
                    .or_insert(frame);
            }
        }
        let training_negatives = cohort
            .members
            .iter()
            .filter_map(|member| self.cegis.cohort_evidence(&member.cohort_id_sha256))
            .flat_map(|(_, negatives)| negatives);
        let mut negatives = BTreeMap::<String, crate::RelationFrame>::new();
        for mut frame in negative_frames.into_values().chain(training_negatives) {
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
        let mut positives = positives.into_values().collect::<Vec<_>>();
        positives.sort_by(|left, right| {
            left.observed_at_unix_nanos
                .cmp(&right.observed_at_unix_nanos)
                .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
        });
        let negatives = negatives.into_values().collect::<Vec<_>>();
        let distinct_sessions = positives
            .iter()
            .map(|frame| frame.session_id_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        let distinct_surfaces = positives
            .iter()
            .filter_map(crate::relation_frame_structural_family_id)
            .collect::<BTreeSet<_>>()
            .len();
        Some(TeacherPoolSnapshot {
            teacher_signature_sha256: winner.teacher_signature_sha256.clone(),
            action_symbol: winner.action_symbol.clone(),
            positive_rows: u64::try_from(positives.len()).unwrap_or(u64::MAX),
            negative_rows: u64::try_from(negatives.len()).unwrap_or(u64::MAX),
            positive_tokens: positives
                .iter()
                .map(|frame| frame.estimated_input_tokens)
                .sum(),
            negative_tokens: negatives
                .iter()
                .map(|frame| frame.estimated_input_tokens)
                .sum(),
            distinct_surfaces,
            distinct_sessions,
            positives,
            negatives,
            invariants,
        })
    }

    fn refresh_opportunity_search_for_signatures(&mut self, signatures: &BTreeSet<String>) {
        self.refresh_opportunity_search_filtered(Some(signatures));
    }

    fn refresh_opportunity_search_filtered(&mut self, signatures: Option<&BTreeSet<String>>) {
        let report = self.cegis.report();
        let mut by_teacher = BTreeMap::<String, (u64, u64, usize, bool, Option<String>)>::new();
        for pool in report.pools {
            if signatures.is_some_and(|selected| !selected.contains(&pool.teacher_signature_sha256))
            {
                continue;
            }
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

fn generation_support_parity_complete(
    generation: &FrozenGeneration,
    parity_ids: &BTreeSet<String>,
    policy: RolloverPolicy,
) -> bool {
    let receipt_backed = generation
        .support
        .iter()
        .filter(|frame| parity_ids.contains(&frame.frame_id_sha256))
        .count();
    support_partition_complete(receipt_backed, policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cross_pool_negative_refresh_is_bounded_to_batch_boundary() {
        assert!(!cross_pool_negative_refresh_due(0));
        assert!(!cross_pool_negative_refresh_due(63));
        assert!(cross_pool_negative_refresh_due(64));
        assert!(!cross_pool_negative_refresh_due(65));
        assert!(cross_pool_negative_refresh_due(128));
    }

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

    fn continuation_frame(
        index: usize,
        function_name: &str,
        argument_name: &str,
        prefix: &str,
        tool_kind: &str,
        accepted: bool,
    ) -> crate::RelationFrame {
        crate::RelationFrame {
            schema: crate::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: format!("{:064x}", index + 100_000),
            event_id_sha256: format!("{:064x}", index + 200_000),
            client_intent_id_sha256: format!("{:064x}", index + 300_000),
            session_id_sha256: format!("{:064x}", index + 400_000),
            observed_at_unix_nanos: u64::try_from(index + 1).unwrap_or(u64::MAX),
            estimated_input_tokens: 100,
            extractor_version: crate::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(accepted),
            atoms: vec![
                crate::RelationAtom::ToolKind {
                    value: tool_kind.to_owned(),
                },
                crate::RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                crate::RelationAtom::TypedSlot {
                    slot_id: 1,
                    value_type: crate::AtomValueType::Identifier,
                    source: crate::AtomSource::Observation,
                    value_sha256: "a".repeat(64),
                },
                crate::RelationAtom::UniqueSlot { slot_id: 1 },
                crate::RelationAtom::ObservationSelector {
                    slot_id: 1,
                    selector: crate::ResponseValueSelector::ContentLinePrefix {
                        prefix: prefix.to_owned(),
                        value_type: crate::AtomValueType::Identifier,
                    },
                },
                crate::RelationAtom::TypedSlot {
                    slot_id: 2,
                    value_type: crate::AtomValueType::Identifier,
                    source: crate::AtomSource::Action,
                    value_sha256: "a".repeat(64),
                },
                crate::RelationAtom::SlotEquality {
                    left_slot: 1,
                    right_slot: 2,
                },
                crate::RelationAtom::ActionFunction {
                    value: function_name.to_owned(),
                },
                crate::RelationAtom::ActionRoleArgument {
                    name: argument_name.to_owned(),
                    slot_id: 2,
                    value_type: Some(crate::AtomValueType::Identifier),
                },
            ],
            evidence_ref_sha256: format!("{:064x}", index + 500_000),
        }
    }

    fn continuation_transition(
        index: usize,
        function_name: &str,
        argument_name: &str,
        prefix: &str,
        request_text: &str,
    ) -> crate::TeacherTransition {
        let frame = continuation_frame(index, function_name, argument_name, prefix, "exec", true);
        let provider_payload = json!({
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "input_text", "text": request_text}]
                },
                {
                    "type": "function_call_output",
                    "output": format!("{prefix}handle-{index}")
                }
            ]
        });
        let program = crate::ResponseProgram::function_call_from_roles(
            function_name,
            crate::ResponseValueSelector::ContentLinePrefix {
                prefix: prefix.to_owned(),
                value_type: crate::AtomValueType::Identifier,
            },
            vec![crate::ResponseArgument::Role {
                name: argument_name.to_owned(),
                role: crate::SemanticRole::ContinuationHandle,
                value_type: Some(crate::AtomValueType::Identifier),
            }],
        );
        let expected = crate::execute_response(&program, request_text, &provider_payload);
        assert_eq!(expected.status, crate::ResponseExecutionStatus::Executed);
        let mut transition =
            crate::teacher_transition_from_completed(&frame, None).expect("teacher transition");
        transition.runtime_parity_case = Some(crate::RuntimeParityCase {
            evidence_ref_sha256: String::new(),
            capture_receipt: None,
            request_text: request_text.to_owned(),
            provider_payload,
            expected_response: expected.response.expect("exact response"),
        });
        transition
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
    fn frozen_support_requires_complete_runtime_parity() {
        let generation = generation();
        let mut parity_ids = generation
            .support
            .iter()
            .take(31)
            .map(|frame| frame.frame_id_sha256.clone())
            .collect::<BTreeSet<_>>();

        assert!(!generation_support_parity_complete(
            &generation,
            &parity_ids,
            RolloverPolicy::default()
        ));
        parity_ids.insert(generation.support[31].frame_id_sha256.clone());
        assert!(generation_support_parity_complete(
            &generation,
            &parity_ids,
            RolloverPolicy::default()
        ));
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
    fn stale_frozen_partition_schedules_one_migration_slice() {
        let mut state = StreamingSelfTrainingState::new(0);
        let mut stale = generation();
        stale.partition_version = FROZEN_PARTITION_VERSION.saturating_sub(1);
        state
            .generations
            .insert(stale.cohort_id_sha256.clone(), stale);
        assert!(state.has_pending_work());

        state
            .generations
            .values_mut()
            .for_each(|generation| generation.partition_version = FROZEN_PARTITION_VERSION);
        assert!(!state.has_pending_work());
    }

    #[test]
    fn effect_law_migration_moves_old_live_parity_to_support_only() {
        let mut state = StreamingSelfTrainingState::new(0);
        for index in 0..32 {
            state
                .observe_transition(&continuation_transition(
                    index,
                    "wait",
                    "cell_id",
                    "Script running with cell ID ",
                    "continue",
                ))
                .expect("live transition");
        }
        state
            .generations
            .insert(generation().cohort_id_sha256.clone(), generation());
        assert_eq!(state.runtime_parity_cases.len(), 32);

        state.prepare_effect_law_migration();

        assert_eq!(state.schema, SELF_TRAINING_STATE_SCHEMA_V3);
        assert!(state.runtime_parity_cases.is_empty());
        assert_eq!(state.replay_support_parity_cases.len(), 32);
        assert!(state.generations.is_empty());
        let alias = state.discovery.semantic_alias_graph().report();
        assert_eq!(alias.rows_seen, 32);
        assert!(alias.accounting_complete);
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
                capture_receipt: None,
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

    #[test]
    fn parity_reservoir_is_bounded_across_live_and_replay() {
        let mut state = StreamingSelfTrainingState::default();
        for index in 0..1_100 {
            let frame = frame(index);
            let frame_id = frame.frame_id_sha256.clone();
            state.replay_support_parity_cases.insert(
                frame_id.clone(),
                crate::RuntimeParityCase {
                    evidence_ref_sha256: frame_id.clone(),
                    capture_receipt: None,
                    request_text: "replay".to_owned(),
                    provider_payload: json!({"index": index}),
                    expected_response: "{}".to_owned(),
                },
            );
            state.replay_support_parity_frames.insert(frame_id, frame);
        }
        for index in 1_100..1_200 {
            let frame = frame(index);
            let frame_id = frame.frame_id_sha256.clone();
            state.runtime_parity_cases.insert(
                frame_id.clone(),
                crate::RuntimeParityCase {
                    evidence_ref_sha256: frame_id.clone(),
                    capture_receipt: None,
                    request_text: "live".to_owned(),
                    provider_payload: json!({"index": index}),
                    expected_response: "{}".to_owned(),
                },
            );
            state.runtime_parity_frames.insert(frame_id, frame);
        }

        state.enforce_parity_reservoir_limit();

        assert_eq!(state.runtime_parity_cases.len(), 32);
        assert!(state.replay_support_parity_cases.is_empty());
        assert_eq!(
            state.runtime_parity_cases.len(),
            state.runtime_parity_frames.len()
        );
        assert_eq!(
            state.replay_support_parity_cases.len(),
            state.replay_support_parity_frames.len()
        );
    }

    #[test]
    fn restored_state_replaces_raw_parity_frame_with_canonical_discovery_frame() {
        let mut canonical = frame(42);
        canonical.atoms = vec![
            crate::RelationAtom::ActionFunction {
                value: "wait".to_owned(),
            },
            crate::RelationAtom::ActionRoleArgument {
                name: "cell_id".to_owned(),
                slot_id: 7,
                value_type: Some(crate::AtomValueType::Integer),
            },
        ];
        let transition = crate::teacher_transition_from_completed(&canonical, None)
            .expect("canonical teacher transition");
        let canonical_training = transition.as_training_relation_frame();
        let mut state = StreamingSelfTrainingState::default();
        assert_eq!(state.discovery.observe_transition(&transition), Ok(true));

        let mut raw = canonical_training.clone();
        for atom in &mut raw.atoms {
            if let crate::RelationAtom::ActionRoleArgument { value_type, .. } = atom {
                *value_type = None;
            }
        }
        assert_ne!(
            crate::teacher_program_signature(&raw),
            crate::teacher_program_signature(&canonical_training)
        );
        state.runtime_parity_cases.insert(
            canonical_training.frame_id_sha256.clone(),
            crate::RuntimeParityCase {
                evidence_ref_sha256: canonical_training.frame_id_sha256.clone(),
                capture_receipt: None,
                request_text: "continue".to_owned(),
                provider_payload: json!({"input": []}),
                expected_response: "{}".to_owned(),
            },
        );
        state
            .runtime_parity_frames
            .insert(canonical_training.frame_id_sha256.clone(), raw);

        state.repair_missing_synthesis_state();

        assert_eq!(
            state
                .runtime_parity_frames
                .get(&canonical_training.frame_id_sha256),
            Some(&canonical_training)
        );
    }

    #[test]
    fn teacher_signature_migration_rekeys_replay_before_schema_enrichment() {
        let mut transition = continuation_transition(
            43,
            "wait",
            "cell_id",
            "Script running with cell ID ",
            "continue",
        );
        for atom in &mut transition.outcome.action.atoms {
            if let crate::RelationAtom::ActionRoleArgument { value_type, .. } = atom {
                *value_type = None;
            }
        }
        let canonical = transition.as_training_relation_frame();
        let canonical_id = canonical.frame_id_sha256.clone();
        let mut state = StreamingSelfTrainingState::default();
        assert_eq!(state.discovery.observe_transition(&transition), Ok(true));

        let stale_id = "f".repeat(64);
        let mut stale_frame = canonical.clone();
        stale_frame.frame_id_sha256.clone_from(&stale_id);
        let mut parity = transition
            .runtime_parity_case
            .take()
            .expect("runtime parity case");
        parity.evidence_ref_sha256.clone_from(&stale_id);
        state
            .replay_support_parity_cases
            .insert(stale_id.clone(), parity);
        state
            .replay_support_parity_frames
            .insert(stale_id.clone(), stale_frame);

        state
            .prepare_teacher_signature_migration()
            .expect("teacher signature migration");

        assert!(
            state
                .replay_support_parity_cases
                .contains_key(&canonical_id)
        );
        assert!(!state.replay_support_parity_cases.contains_key(&stale_id));
        let migrated = state
            .discovery
            .pool_snapshots()
            .into_iter()
            .flat_map(|pool| pool.positives)
            .find(|frame| frame.frame_id_sha256 == canonical_id)
            .expect("migrated canonical frame");
        assert!(migrated.atoms.iter().any(|atom| matches!(
            atom,
            crate::RelationAtom::ActionRoleArgument {
                name,
                value_type: Some(crate::AtomValueType::String),
                ..
            } if name == "cell_id"
        )));
    }

    #[test]
    fn semantic_law_cohort_combines_two_verified_physical_adapters() {
        let mut state = StreamingSelfTrainingState::new(0);
        for index in 0..16 {
            let negative = continuation_frame(
                index,
                "cancel",
                "handle",
                "Cancelled handle ",
                "negative",
                false,
            );
            let transition = crate::teacher_transition_from_completed(&negative, None)
                .expect("negative teacher transition");
            state
                .observe_transition(&transition)
                .expect("observe negative");
        }
        for index in 0..32 {
            let transition = continuation_transition(
                1_000 + index,
                "wait",
                "cell_id",
                "Script running with cell ID ",
                "wait for script",
            );
            state
                .observe_transition(&transition)
                .expect("observe wait adapter");
        }
        for index in 0..32 {
            let transition = continuation_transition(
                2_000 + index,
                "continue_process",
                "session",
                "Process running with session ID ",
                "continue process",
            );
            state
                .observe_transition(&transition)
                .expect("observe process adapter");
        }
        for _ in 0..2_048 {
            if state.run_work_slice() == 0 && !state.has_pending_work() {
                break;
            }
        }

        let exact = state.cegis.winners();
        assert_eq!(exact.len(), 2);
        let law = state
            .discovery
            .semantic_law_signature(&exact[0].teacher_signature_sha256)
            .expect("first semantic law");
        assert_eq!(
            Some(law.as_str()),
            state
                .discovery
                .semantic_law_signature(&exact[1].teacher_signature_sha256)
                .as_deref()
        );
        let semantic = state
            .build_semantic_law_cohort(&law, &exact)
            .unwrap_or_else(|blocker| panic!("semantic law cohort: {blocker}"));
        assert_eq!(semantic.member_signatures.len(), 2);
        assert!(matches!(
            semantic.winner.program.operation,
            crate::ResponseOperation::UniqueConsensus {
                adapter_wave: None,
                ..
            }
        ));
        let pool = state
            .cohort_pool_snapshot(&semantic)
            .expect("combined law pool");
        assert!(pool.positives.len() >= 64);
        assert!(pool.negatives.len() >= 16);

        state.refresh_generations_filtered(None);
        let frozen = state.report(0);
        assert!(frozen.generations.iter().any(|generation| {
            generation.physical_adapter_count == 2
                && generation.support_rows == 32
                && generation.future_rows == 0
        }));

        for index in 0..16 {
            let transition = continuation_transition(
                3_000 + index,
                "wait",
                "cell_id",
                "Script running with cell ID ",
                "wait for script",
            );
            state
                .observe_transition(&transition)
                .expect("observe wait future");
        }
        for index in 0..16 {
            let transition = continuation_transition(
                4_000 + index,
                "continue_process",
                "session",
                "Process running with session ID ",
                "continue process",
            );
            state
                .observe_transition(&transition)
                .expect("observe process future");
        }
        for _ in 0..2_048 {
            if state.run_work_slice() == 0 && !state.has_pending_work() {
                break;
            }
        }
        state.refresh_generations_filtered(None);
        let report = state.report(0);
        assert_eq!(report.semantic_law_cohorts, 1);
        assert_eq!(report.semantic_law_physical_adapters, 2);
        assert!(report.semantic_law_blockers.is_empty());
        assert!(report.generations.iter().any(|generation| {
            generation.physical_adapter_count == 2
                && generation.support_rows == 32
                && generation.future_rows >= 32
                && generation.blocker.is_none()
        }));
        assert_eq!(report.admission_ready_cohorts, 1);
    }
}
