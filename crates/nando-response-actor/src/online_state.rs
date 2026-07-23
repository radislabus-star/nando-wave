use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::rollover::support_partition_complete;
use crate::{
    CegisCoordinator, CegisReport, CegisWinner, CrossSurfaceFamilyDiscovery,
    FROZEN_PARTITION_VERSION, FamilyDiscoveryConfig, FamilyDiscoveryReport, FrozenGeneration,
    OpportunityBoard, RolloverPolicy, TeacherPoolSnapshot, TeacherTransition, VersionSpaceConfig,
    freeze_generation, refresh_frozen_generation,
};

#[path = "online_diagnostics.rs"]
mod diagnostics;
#[path = "online_generation_evidence.rs"]
mod generation_evidence;
#[path = "online_signal_tree.rs"]
mod signal_tree;

use generation_evidence::*;
use signal_tree::build_signal_tree;

pub use diagnostics::{
    SEMANTIC_LAW_EVIDENCE_AUDIT_SCHEMA_V1, SemanticLawActorAudit, SemanticLawActorReplayOutcome,
    SemanticLawEvidenceAudit, SemanticLawEvidenceAuditRow, SemanticLawSelectorCandidate,
    SemanticLawValueOccurrence,
};
pub use nando_operator_learning::{
    MinerSignalStageReport, MinerSignalTreeReport, SELF_TRAINING_STATE_SCHEMA_V2,
    SELF_TRAINING_STATE_SCHEMA_V3, SELF_TRAINING_STATE_SCHEMA_V4, SELF_TRAINING_STATE_SCHEMA_V5,
    SEMANTIC_EVIDENCE_RECEIPT_SCHEMA_V1, SelfTrainingAdmissionCohort, SelfTrainingGenerationReport,
    SelfTrainingStateReport, SemanticEvidenceOutcome, SemanticEvidenceReceipt,
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
    transitions_seen > 0 && transitions_seen.is_multiple_of(CROSS_POOL_NEGATIVE_REFRESH_INTERVAL)
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
    /// Receipt-backed generation evidence is level-triggered independently of
    /// CEGIS. A busy unrelated version space must never strand already routed
    /// live receipts outside generation-owned frozen future storage.
    #[serde(default)]
    dirty_generation_signatures: BTreeSet<String>,
    #[serde(default)]
    runtime_parity_cases: BTreeMap<String, crate::RuntimeParityCase>,
    #[serde(default)]
    runtime_parity_frames: BTreeMap<String, crate::RelationFrame>,
    #[serde(default)]
    replay_support_parity_cases: BTreeMap<String, crate::RuntimeParityCase>,
    #[serde(default)]
    replay_support_parity_frames: BTreeMap<String, crate::RelationFrame>,
    /// Frozen proof ownership is per generation, never per teacher signature.
    /// Signature reservoirs may evict candidates, but they must not mutate an
    /// immutable support root or turn historical support into frozen future.
    #[serde(default)]
    generation_parity_receipts: BTreeMap<String, GenerationParityReceipts>,
}

impl StreamingSelfTrainingState {
    #[must_use]
    pub fn new(now_unix: u64) -> Self {
        Self {
            schema: SELF_TRAINING_STATE_SCHEMA_V5.to_owned(),
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
            dirty_generation_signatures: BTreeSet::new(),
            runtime_parity_cases: BTreeMap::new(),
            runtime_parity_frames: BTreeMap::new(),
            replay_support_parity_cases: BTreeMap::new(),
            replay_support_parity_frames: BTreeMap::new(),
            generation_parity_receipts: BTreeMap::new(),
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
        for generation in self.generations.values() {
            let Some(receipts) = self
                .generation_parity_receipts
                .get(&generation.generation_id_sha256)
            else {
                continue;
            };
            for parity_frame in generation.support.iter().chain(&generation.future) {
                let parity = receipts
                    .support
                    .get(&parity_frame.frame_id_sha256)
                    .or_else(|| receipts.future.get(&parity_frame.frame_id_sha256));
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
        self.schema = SELF_TRAINING_STATE_SCHEMA_V5.to_owned();
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

    /// Re-evaluate learned programs after role-grounding or semantic-adapter changes.
    /// Immutable support/future generations and parity receipts remain owned by
    /// their original generation; only the derived CEGIS search is rebuilt.
    pub fn prepare_derived_program_migration(&mut self) {
        self.schema = SELF_TRAINING_STATE_SCHEMA_V5.to_owned();
        self.cegis.prepare_strategy_migration();
        self.negative_refresh_cursor = None;
        self.rebuild_queue = self
            .discovery
            .pool_snapshots()
            .into_iter()
            .map(|pool| pool.teacher_signature_sha256)
            .collect();
        self.dirty_derived_signatures = self.rebuild_queue.iter().cloned().collect();
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
        self.generation_parity_receipts.clear();
        self.dirty_generation_signatures.clear();
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
        self.generation_parity_receipts.clear();
        self.dirty_generation_signatures.clear();
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
        let cohort_regroup_required = !matches!(
            self.schema.as_str(),
            SELF_TRAINING_STATE_SCHEMA_V4 | SELF_TRAINING_STATE_SCHEMA_V5
        );
        self.schema = SELF_TRAINING_STATE_SCHEMA_V5.to_owned();
        self.repair_parity_frames_from_discovery();
        self.enforce_parity_reservoir_limit();
        self.dirty_generation_signatures.extend(
            self.generations
                .values()
                .map(|generation| generation.teacher_signature_sha256.clone()),
        );
        self.refresh_dirty_generation_evidence(None);
        if cohort_regroup_required {
            // Cohort grouping is derived from already verified winners and bounded
            // parity receipts. Recompute it once on schema upgrade so a checkpoint
            // does not wait for a future edge-trigger from every existing signature.
            let winner_signatures = self
                .cegis
                .winners()
                .into_iter()
                .map(|winner| winner.teacher_signature_sha256)
                .collect::<BTreeSet<_>>();
            self.refresh_generations_for_signatures(&winner_signatures);
        }
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
        // Reconciliation must be level-triggered. A checkpoint may contain a
        // healthy CEGIS arena for some teacher signatures while newer discovery
        // pools have never crossed an edge-triggered refresh boundary. Queue only
        // those missing signatures; never clear existing generations or receipts.
        let represented_signatures = self.cegis.teacher_signatures();
        let mut queued = self.rebuild_queue.iter().cloned().collect::<BTreeSet<_>>();
        for pool in self.discovery.pool_snapshots() {
            if !represented_signatures.contains(&pool.teacher_signature_sha256)
                && queued.insert(pool.teacher_signature_sha256.clone())
            {
                self.rebuild_queue
                    .push_back(pool.teacher_signature_sha256.clone());
                self.dirty_derived_signatures
                    .insert(pool.teacher_signature_sha256);
            }
        }
        self.discovery
            .semantic_alias_graph_mut()
            .reopen_under_evidenced_rejections(self.rollover_policy.support_rows);
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
        let stale_partition_signatures = self
            .generations
            .values()
            .filter(|generation| generation.partition_version < FROZEN_PARTITION_VERSION)
            .map(|generation| generation.teacher_signature_sha256.clone())
            .collect::<BTreeSet<_>>();
        self.dirty_derived_signatures
            .extend(stale_partition_signatures.iter().cloned());
        if !stale_partition_signatures.is_empty() && !self.cegis.is_empty() {
            // Checkpoint schema migration cannot wait for global synthesis
            // quiescence: a live stream may keep CEGIS busy indefinitely.
            self.refresh_generations_for_signatures(&stale_partition_signatures);
            for signature in &stale_partition_signatures {
                let still_stale = self.generations.values().any(|generation| {
                    generation.teacher_signature_sha256 == *signature
                        && generation.partition_version < FROZEN_PARTITION_VERSION
                });
                if !still_stale {
                    self.dirty_derived_signatures.remove(signature);
                }
            }
        }
        if self.discovery.teacher_pool_count() == 0 || !self.cegis.is_empty() {
            return;
        }
        self.cegis.prepare_strategy_migration();
        self.generations.clear();
        self.generation_parity_receipts.clear();
        self.dirty_generation_signatures.clear();
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
            let teacher_signature = parity_teacher_signature(&training_frame);
            parity_case.evidence_ref_sha256 = training_frame_id.clone();
            self.runtime_parity_cases
                .insert(training_frame_id.clone(), parity_case);
            self.runtime_parity_frames
                .insert(training_frame_id, training_frame);
            self.enforce_parity_reservoir_limit();
            self.dirty_generation_signatures.insert(teacher_signature);
            self.dirty_generation_signatures
                .insert(transition.outcome.action.signature_sha256.clone());
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

    fn support_parity_case(&self, frame_id: &str) -> Option<crate::RuntimeParityCase> {
        self.runtime_parity_cases
            .get(frame_id)
            .or_else(|| self.replay_support_parity_cases.get(frame_id))
            .cloned()
            .or_else(|| {
                self.generation_parity_receipts
                    .values()
                    .find_map(|receipts| {
                        receipts
                            .support
                            .get(frame_id)
                            .or_else(|| receipts.future.get(frame_id))
                    })
                    .cloned()
            })
    }

    fn future_parity_case(&self, frame_id: &str) -> Option<crate::RuntimeParityCase> {
        self.runtime_parity_cases
            .get(frame_id)
            .cloned()
            .or_else(|| {
                self.generation_parity_receipts
                    .values()
                    .find_map(|receipts| receipts.future.get(frame_id))
                    .cloned()
            })
    }

    fn parity_receipts_for_generation(
        &self,
        generation: &FrozenGeneration,
    ) -> GenerationParityReceipts {
        GenerationParityReceipts {
            support: generation
                .support
                .iter()
                .filter_map(|frame| {
                    self.support_parity_case(&frame.frame_id_sha256)
                        .map(|receipt| (frame.frame_id_sha256.clone(), receipt))
                })
                .collect(),
            future: generation
                .future
                .iter()
                .filter_map(|frame| {
                    self.future_parity_case(&frame.frame_id_sha256)
                        .map(|receipt| (frame.frame_id_sha256.clone(), receipt))
                })
                .collect(),
        }
    }

    /// Continues cold synthesis without requiring another event. The worker
    /// calls this only while queued work exists and always in bounded slices.
    pub fn run_work_slice(&mut self) -> usize {
        self.run_work_slice_with_progress().0
    }

    /// Separates exact-check work from metadata progress so the event-driven
    /// worker can drain finite rebuild/alias queues without mistaking a
    /// zero-check consolidation slice for an idle miner.
    pub fn run_work_slice_with_progress(&mut self) -> (usize, bool) {
        let mut progressed = false;
        let preferred_support_ids = self.parity_support_ids();
        if let Some(signature) = self.rebuild_queue.pop_front() {
            progressed = true;
            if let Some(pool) = self.pool_snapshot_with_parity(&signature) {
                self.cegis
                    .refresh_pool_with_preferred_support(&pool, &preferred_support_ids);
            }
        }
        let checks = self.cegis.run_next_slice();
        if checks > 0 {
            progressed = true;
            self.work_slices_completed = self.work_slices_completed.saturating_add(1);
            self.exact_checks_completed = self
                .exact_checks_completed
                .saturating_add(u64::try_from(checks).unwrap_or(u64::MAX));
        }
        let dirty_generation_count = self.dirty_generation_signatures.len();
        self.refresh_dirty_generation_evidence(None);
        progressed |= self.dirty_generation_signatures.len() != dirty_generation_count;
        if self.rebuild_queue.is_empty() {
            let synthesis_quiescent = !self.cegis.has_pending_work();
            let alias_updates = self.prove_candidate_alias_support(synthesis_quiescent);
            progressed |= alias_updates > 0;
            if alias_updates > 0 || synthesis_quiescent {
                let dirty_derived_count = self.dirty_derived_signatures.len();
                self.refresh_dirty_derived_state(None);
                progressed |= self.dirty_derived_signatures.len() != dirty_derived_count;
            }
        }
        (checks, progressed)
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
        self.refresh_dirty_generation_evidence(Some(signatures));
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

    fn refresh_dirty_generation_evidence(&mut self, selected: Option<&BTreeSet<String>>) {
        let signatures = self
            .dirty_generation_signatures
            .iter()
            .filter(|signature| selected.is_none_or(|selected| selected.contains(*signature)))
            .cloned()
            .collect::<BTreeSet<_>>();
        if signatures.is_empty() {
            return;
        }
        self.refresh_generations_for_signatures(&signatures);
        for signature in signatures {
            self.dirty_generation_signatures.remove(&signature);
        }
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
            .chain(
                self.generation_parity_receipts
                    .values()
                    .flat_map(|receipts| receipts.support.keys().chain(receipts.future.keys())),
            )
            .cloned()
            .collect()
    }

    fn preferred_generation_support_ids(&self) -> BTreeSet<String> {
        self.replay_support_parity_cases
            .keys()
            .chain(
                self.generation_parity_receipts
                    .values()
                    .flat_map(|receipts| receipts.support.keys()),
            )
            .cloned()
            .collect()
    }

    fn prove_candidate_alias_support(&mut self, finalize_missing_winners: bool) -> usize {
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
        let ready_signatures = winners_by_signature
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        updates = updates.saturating_add(
            self.discovery
                .semantic_alias_graph_mut()
                .ensure_ready_candidate_forest(&ready_signatures),
        );
        let candidates = self
            .discovery
            .semantic_alias_graph()
            .candidate_edges()
            .cloned()
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return updates;
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
                        || blocker.contains("wrong")
                        || (blocker.contains("unseparable")
                            && !crate::semantic_alias::retryable_adapter_unseparable(
                                &blocker,
                                self.rollover_policy.support_rows,
                            ));
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
            // A single teacher signature may still contain several structural
            // CEGIS cohorts. Keeping those winners separate fragments the same
            // bounded parity reservoir and can prevent every cohort from ever
            // owning complete support. Pool them through the same exact parity
            // proof used for cross-signature semantic aliases; incompatible
            // programs fail closed inside build_semantic_law_cohort.
            let derived =
                (members.len() >= 2).then(|| self.build_semantic_law_cohort(&law, &members));
            match derived {
                Some(Ok(derived)) => cohorts.push(derived),
                Some(Err(blocker)) => {
                    *blockers.entry(blocker).or_default() += 1;
                    cohorts.extend(members.into_iter().map(|winner| {
                        let law_signature_sha256 = self
                            .discovery
                            .semantic_law_signature(&winner.teacher_signature_sha256)
                            .unwrap_or_else(|| law.clone());
                        DerivedWinnerCohort {
                            member_signatures: BTreeSet::from([winner
                                .teacher_signature_sha256
                                .clone()]),
                            physical_adapter_count: 1,
                            members: vec![winner.clone()],
                            law_signature_sha256,
                            winner,
                        }
                    }));
                }
                None => {
                    cohorts.extend(members.into_iter().map(|winner| {
                        let law_signature_sha256 = self
                            .discovery
                            .semantic_law_signature(&winner.teacher_signature_sha256)
                            .unwrap_or_else(|| law.clone());
                        DerivedWinnerCohort {
                            member_signatures: BTreeSet::from([winner
                                .teacher_signature_sha256
                                .clone()]),
                            physical_adapter_count: 1,
                            members: vec![winner.clone()],
                            law_signature_sha256,
                            winner,
                        }
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
        let discovered_laws = members
            .iter()
            .filter_map(|winner| {
                self.discovery
                    .semantic_law_signature(&winner.teacher_signature_sha256)
            })
            .collect::<BTreeSet<_>>();
        if discovered_laws.len() > 1 {
            return Err(format!(
                "semantic_law_members_cross_multiple_laws:{}",
                discovered_laws.len()
            ));
        }
        let canonical_law_signature = discovered_laws
            .into_iter()
            .next()
            .unwrap_or_else(|| law_signature.to_owned());
        let member_signatures = members
            .iter()
            .map(|winner| winner.teacher_signature_sha256.clone())
            .collect::<BTreeSet<_>>();
        if members.len() < 2 {
            return Err("semantic_law_members_below_two".to_owned());
        }
        let mut program_classes =
            BTreeMap::<String, (crate::ResponseProgram, Vec<CegisWinner>)>::new();
        for winner in members {
            let program = winner.program.clone();
            let digest = crate::sha256_bytes(
                &serde_json::to_vec(&program)
                    .map_err(|error| format!("semantic_law_program_encode:{error}"))?,
            );
            program_classes
                .entry(digest)
                .or_insert_with(|| (program, Vec::new()))
                .1
                .push(winner.clone());
        }
        let mut parity_rows_by_id = BTreeMap::new();
        for signature in &member_signatures {
            // Semantic regrouping must see immutable generation-owned support;
            // the bounded discovery reservoir may already have evicted those
            // frames. They remain support-only and never become fresh future.
            let Some(pool) = self.pool_snapshot_with_parity(signature) else {
                continue;
            };
            for frame in pool.positives {
                if crate::teacher_semantic_law_signature(&frame).as_deref()
                    != Some(canonical_law_signature.as_str())
                {
                    continue;
                }
                let Some(case) = self.support_parity_case(&frame.frame_id_sha256) else {
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
        let member_parity_rows = parity_rows
            .iter()
            .filter(|(signature, frame, _)| {
                semantic_member_action_matches(
                    members,
                    &canonical_law_signature,
                    Some(signature.as_str()),
                    frame,
                )
            })
            .collect::<Vec<_>>();
        if member_parity_rows.len() != parity_rows.len() {
            return Err(format!(
                "semantic_law_unowned_parity_rows:{}:{}",
                member_parity_rows.len(),
                parity_rows.len()
            ));
        }
        let member_parity_cases = member_parity_rows
            .iter()
            .map(|(_, _, parity)| parity)
            .collect::<Vec<_>>();
        let mut canonical_receipt_support_rows = 0_usize;
        let mut canonical_grounded_rows = 0_usize;
        let mut canonical_eligible_programs = 0_usize;
        let mut canonicalized_programs = 0_usize;
        for winner in members {
            let receipt_backed_support = parity_rows
                .iter()
                .filter(|(signature, _, _)| signature == &winner.teacher_signature_sha256)
                .map(|(_, frame, _)| frame.clone())
                .collect::<Vec<_>>();
            if receipt_backed_support.len() < self.rollover_policy.support_rows {
                continue;
            }
            canonical_receipt_support_rows =
                canonical_receipt_support_rows.saturating_add(receipt_backed_support.len());
            canonical_grounded_rows = canonical_grounded_rows.saturating_add(
                receipt_backed_support
                    .iter()
                    .filter(|frame| {
                        crate::ground_roles(frame).iter().any(|hypothesis| {
                            hypothesis
                                .bindings
                                .contains_key(&crate::SemanticRole::ContinuationHandle)
                        })
                    })
                    .count(),
            );
            canonical_eligible_programs =
                canonical_eligible_programs.saturating_add(usize::from(matches!(
                    &winner.program.operation,
                    crate::ResponseOperation::FunctionCallFromRoles { .. }
                        | crate::ResponseOperation::CustomToolCallFromRoles { .. }
                )));
            // Legacy discovery pools may contain incomplete frames that predate
            // role extraction. Runtime parity receipts are the bounded, exact
            // support authority for broadening a physical selector into a role.
            let Some(program) = crate::synthesis::canonicalize_continuation_role_program(
                &winner.program,
                &receipt_backed_support,
            ) else {
                continue;
            };
            canonicalized_programs = canonicalized_programs.saturating_add(1);
            let digest = crate::sha256_bytes(
                &serde_json::to_vec(&program)
                    .map_err(|error| format!("semantic_law_program_encode:{error}"))?,
            );
            program_classes
                .entry(digest)
                .or_insert_with(|| (program, Vec::new()))
                .1
                .push(winner.clone());
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
            let mut positive_layouts = BTreeSet::new();
            let mut wrong_layouts = BTreeSet::new();
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
                    if let Ok(layout) =
                        crate::runtime::actor_structural_layout_sha256(&case.provider_payload)
                    {
                        positive_layouts.insert(layout);
                    }
                    positives.push(atoms);
                } else {
                    if execution.status == crate::ResponseExecutionStatus::Executed {
                        adapter_wrong_rows = adapter_wrong_rows.saturating_add(1);
                        if let Ok(layout) =
                            crate::runtime::actor_structural_layout_sha256(&case.provider_payload)
                        {
                            wrong_layouts.insert(layout);
                        }
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
                .map(|atoms| nando_operator_learning::adapter_wave_atom_fingerprint(atoms))
                .collect::<BTreeSet<_>>();
            let clean_positives = positives
                .into_iter()
                .filter(|atoms| {
                    !negative_fingerprints.contains(
                        &nando_operator_learning::adapter_wave_atom_fingerprint(atoms),
                    )
                })
                .collect::<Vec<_>>();
            if clean_positives.is_empty() {
                return Err("semantic_law_adapter_without_clean_positive_parity".to_owned());
            }
            let allowed_layout_sha256 = positive_layouts
                .difference(&wrong_layouts)
                .take(crate::program::MAX_CONSENSUS_LAYOUTS)
                .cloned()
                .collect::<Vec<_>>();
            let variant = crate::ResponseConsensusVariant {
                program: program.clone(),
                allowed_layout_sha256,
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
            semantic_program_covers_all_runtime_parity(candidate, &member_parity_cases)
        });
        let physical_adapter_count = member_signatures.len();
        let program = if let Some(program) = exact_only_program {
            program
        } else if variants.len() == 1 {
            variants
                .pop()
                .ok_or_else(|| "semantic_law_variant_missing".to_owned())?
                .program
        } else {
            let exact_consensus = crate::ResponseProgram::unique_consensus(variants.clone());
            let exact_resolves_all =
                semantic_program_covers_all_runtime_parity(&exact_consensus, &member_parity_cases);
            if exact_resolves_all {
                exact_consensus
            } else {
                let routes = adapter_training
                    .iter()
                    .map(|(positives, negatives)| {
                        nando_operator_learning::fit_adapter_wave_route(positives, negatives, 16)
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
        let mut canonical_execution_reasons = BTreeMap::<String, usize>::new();
        for (_, frame, parity) in &parity_rows {
            let execution =
                crate::execute_response(&program, &parity.request_text, &parity.provider_payload);
            if execution.status != crate::ResponseExecutionStatus::Executed {
                *canonical_execution_reasons
                    .entry(execution.reason.clone())
                    .or_default() += 1;
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
        if accepted_parity_rows != member_parity_cases.len() {
            // Keep this diagnostic structural and bounded: runtime reasons are
            // static actor/verifier labels, never request or provider payloads.
            let canonical_execution_reasons = canonical_execution_reasons
                .into_iter()
                .take(8)
                .map(|(reason, rows)| format!("{reason}={rows}"))
                .collect::<Vec<_>>()
                .join(",");
            return Err(format!(
                "semantic_law_clean_parity_rows_below_{}:{}:programs={}:variants={}:exact={adapter_exact_rows}:wrong={adapter_wrong_rows}:abstain={adapter_abstain_rows}:same_name={adapter_same_name_rows}:same_arguments={adapter_same_arguments_rows}:same_name_and_arguments={adapter_same_name_and_arguments_rows}:canonical_support={canonical_receipt_support_rows}:canonical_grounded={canonical_grounded_rows}:canonical_eligible={canonical_eligible_programs}:canonicalized={canonicalized_programs}:reasons={canonical_execution_reasons}",
                member_parity_cases.len(),
                accepted_parity_rows,
                program_classes.len(),
                variants.len(),
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
        // Physical CEGIS cohorts are evidence for a semantic law, not part of
        // its identity. Including their IDs here made an unchanged program
        // acquire a new cohort ID whenever another surface was discovered,
        // stranding its immutable frozen-future window before admission.
        let cohort_id_sha256 = crate::sha256_bytes(
            &serde_json::to_vec(&("nando.semantic-law-cohort.v2", law_signature, &program))
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
            law_signature_sha256: canonical_law_signature,
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
            .chain(
                self.generations
                    .values()
                    .flat_map(|generation| generation.support.iter().chain(&generation.future)),
            )
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
            || !self.dirty_generation_signatures.is_empty()
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
            || self
                .dirty_generation_signatures
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
                let generation_frames = generation
                    .support
                    .iter()
                    .chain(&generation.future)
                    .cloned()
                    .collect::<Vec<_>>();
                let classified = self.classified_cohort_pool(derived, &generation_frames)?;
                let winner_program_sha256 = crate::canonical_json_sha256(&winner.program).ok()?;
                let mut semantic_evidence_receipts = classified
                    .evidence
                    .iter()
                    .map(|row| SemanticEvidenceReceipt {
                        schema: SEMANTIC_EVIDENCE_RECEIPT_SCHEMA_V1.to_owned(),
                        generation_id_sha256: generation.generation_id_sha256.clone(),
                        cohort_id_sha256: winner.cohort_id_sha256.clone(),
                        winner_program_sha256: winner_program_sha256.clone(),
                        frame_id_sha256: row.frame.frame_id_sha256.clone(),
                        evidence_ref_sha256: row.frame.evidence_ref_sha256.clone(),
                        outcome: row.outcome,
                        reason: row.reason.to_owned(),
                    })
                    .collect::<Vec<_>>();
                semantic_evidence_receipts
                    .sort_by(|left, right| left.frame_id_sha256.cmp(&right.frame_id_sha256));
                let receipts = self
                    .generation_parity_receipts
                    .get(&generation.generation_id_sha256)?;
                Some(SelfTrainingAdmissionCohort {
                    winner,
                    physical_members: derived.members.clone(),
                    pool: classified.pool,
                    generation: generation.clone(),
                    semantic_evidence_receipts,
                    semantic_alias_edges: self
                        .discovery
                        .semantic_alias_graph()
                        .proven_edges_for_members(&derived.member_signatures),
                    runtime_parity_cases: generation
                        .support
                        .iter()
                        .chain(generation.future.iter())
                        .filter_map(|frame| {
                            receipts
                                .support
                                .get(&frame.frame_id_sha256)
                                .or_else(|| receipts.future.get(&frame.frame_id_sha256))
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
                        self.support_parity_case(&frame.frame_id_sha256).is_some()
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
                let mut diagnostic = FutureFilterDiagnostic {
                    matching_rows: matching.len(),
                    matching_sessions,
                    post_repair_rows: post_repair.len(),
                    post_repair_sessions,
                    live_rows: live.len(),
                    after_watermark_rows: after_watermark.len(),
                    ..FutureFilterDiagnostic::default()
                };
                // Keep these outcomes mutually exclusive. The accounting identity makes
                // an overly broad session/intent identity visible instead of silently
                // presenting it as a generic lack of frozen-future evidence.
                for frame in after_watermark {
                    if support_ids.contains(frame.frame_id_sha256.as_str())
                        || winner.support_frame_ids.contains(&frame.frame_id_sha256)
                    {
                        diagnostic.support_frame_rejects += 1;
                    } else if support_sessions.contains(frame.session_id_sha256.as_str()) {
                        diagnostic.support_session_rejects += 1;
                    } else if support_intents.contains(frame.client_intent_id_sha256.as_str()) {
                        diagnostic.support_intent_rejects += 1;
                    } else if support_events.contains(frame.event_id_sha256.as_str()) {
                        diagnostic.support_event_rejects += 1;
                    } else {
                        diagnostic.independent_rows += 1;
                        if !crate::synthesis::program_is_consistent(&winner.program, frame) {
                            diagnostic.program_mismatch_rejects += 1;
                        } else {
                            diagnostic.consistent_rows += 1;
                            if crate::cegis::winner_routes_frame(winner, frame) {
                                diagnostic.routed_rows += 1;
                            } else {
                                diagnostic.route_mismatch_rejects += 1;
                            }
                        }
                    }
                }
                Some((cohort_id.clone(), diagnostic))
            })
            .collect::<BTreeMap<_, _>>();
        let mut generations = self
            .generations
            .values()
            .map(|generation| {
                let receipts = self
                    .generation_parity_receipts
                    .get(&generation.generation_id_sha256);
                let parity = parity_diagnostics
                    .get(&generation.cohort_id_sha256)
                    .copied()
                    .unwrap_or_default();
                SelfTrainingGenerationReport {
                    partition_version: generation.partition_version,
                    generation_id_sha256: generation.generation_id_sha256.clone(),
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
                    support_watermark_unix_nanos: generation.support_watermark_unix_nanos,
                    support_rows: generation.support.len(),
                    support_sessions: generation
                        .support
                        .iter()
                        .map(|frame| frame.session_id_sha256.as_str())
                        .collect::<BTreeSet<_>>()
                        .len(),
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
                            receipts.is_some_and(|receipts| {
                                receipts.support.contains_key(&frame.frame_id_sha256)
                            })
                        })
                        .count(),
                    support_runtime_parity_tokens: generation
                        .support
                        .iter()
                        .filter(|frame| {
                            receipts.is_some_and(|receipts| {
                                receipts.support.contains_key(&frame.frame_id_sha256)
                            })
                        })
                        .map(|frame| frame.estimated_input_tokens)
                        .sum(),
                    matching_runtime_parity_rows: parity.matching_rows,
                    matching_runtime_parity_sessions: parity.matching_sessions,
                    post_repair_runtime_parity_rows: parity.post_repair_rows,
                    post_repair_runtime_parity_sessions: parity.post_repair_sessions,
                    live_runtime_parity_rows: parity.live_rows,
                    after_future_watermark_rows: parity.after_watermark_rows,
                    support_frame_rejects: parity.support_frame_rejects,
                    support_session_rejects: parity.support_session_rejects,
                    support_intent_rejects: parity.support_intent_rejects,
                    support_event_rejects: parity.support_event_rejects,
                    independent_future_rows: parity.independent_rows,
                    program_mismatch_rejects: parity.program_mismatch_rejects,
                    program_consistent_future_rows: parity.consistent_rows,
                    route_mismatch_rejects: parity.route_mismatch_rejects,
                    routed_future_rows: parity.routed_rows,
                    runtime_parity_rows: generation
                        .future
                        .iter()
                        .filter(|frame| {
                            receipts.is_some_and(|receipts| {
                                receipts.future.contains_key(&frame.frame_id_sha256)
                            })
                        })
                        .count(),
                    runtime_parity_tokens: generation
                        .future
                        .iter()
                        .filter(|frame| {
                            receipts.is_some_and(|receipts| {
                                receipts.future.contains_key(&frame.frame_id_sha256)
                            })
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
        let mut parity_rows_by_teacher_signature = BTreeMap::new();
        for (frame_id, frame) in &parity_frames {
            let Some(discovery_frame) = discovery_frames.get(*frame_id) else {
                continue;
            };
            if crate::teacher_program_signature(frame)
                != crate::teacher_program_signature(discovery_frame)
            {
                continue;
            }
            let Some(signature) = crate::teacher_program_signature(discovery_frame) else {
                continue;
            };
            *parity_rows_by_teacher_signature
                .entry(signature)
                .or_default() += 1;
        }
        let signal_tree = build_signal_tree(
            self.transitions_seen,
            &discovery,
            &cegis,
            &generations,
            admission_ready_cohorts,
        );
        SelfTrainingStateReport {
            schema: SELF_TRAINING_STATE_SCHEMA_V5.to_owned(),
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
            parity_rows_by_teacher_signature,
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
        let all_winners = self.derived_winner_cohorts();
        // Incremental regrouping can replace a semantic cohort ID when its
        // physical adapter set changes. Prune an orphan only when a live cohort
        // covers the same teacher/member signature; absence alone may mean a
        // partially restored CEGIS checkpoint whose proof must be preserved.
        let live_ids = all_winners
            .iter()
            .map(|cohort| cohort.winner.cohort_id_sha256.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let live_signatures = all_winners
            .iter()
            .flat_map(|cohort| {
                std::iter::once(cohort.winner.teacher_signature_sha256.as_str())
                    .chain(cohort.member_signatures.iter().map(String::as_str))
            })
            .collect::<std::collections::BTreeSet<_>>();
        self.generations.retain(|cohort_id, generation| {
            live_ids.contains(cohort_id.as_str())
                || !live_signatures.contains(generation.teacher_signature_sha256.as_str())
        });
        let winners = all_winners
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
        let support_eligible_ids = self.parity_support_ids();
        let preferred_generation_support_ids = self.preferred_generation_support_ids();
        let live_future_eligible_ids = self
            .runtime_parity_cases
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
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
                    self.generation_parity_receipts
                        .get(&current.generation_id_sha256),
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
                        &live_future_eligible_ids,
                        &preferred_generation_support_ids,
                    )
                });
                let proof_repartition_improves = incomplete_support_evidence
                    && refrozen.as_ref().is_some_and(|candidate| {
                        support_partition_complete(candidate.support.len(), self.rollover_policy)
                            && candidate.support != current.support
                    });
                let partition_upgrade_improves = partition_upgrade
                    && refrozen.as_ref().is_some_and(|candidate| {
                        support_partition_complete(candidate.support.len(), self.rollover_policy)
                    });
                let incomplete_support_improves = incomplete_support
                    && refrozen
                        .as_ref()
                        .is_some_and(|candidate| candidate.support.len() > current.support.len());
                let repartition_improves = partition_upgrade_improves
                    || incomplete_support_improves
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
                    let mut future_eligible_ids = live_future_eligible_ids.clone();
                    if let Some(receipts) = self
                        .generation_parity_receipts
                        .get(&current.generation_id_sha256)
                    {
                        future_eligible_ids.extend(receipts.future.keys().cloned());
                    }
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
                        &live_future_eligible_ids,
                        &preferred_generation_support_ids,
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
                let previous_generation_id = self
                    .generations
                    .get(&winner.cohort_id_sha256)
                    .map(|generation| generation.generation_id_sha256.clone());
                let receipts = self.parity_receipts_for_generation(&next);
                self.generations
                    .insert(winner.cohort_id_sha256.clone(), next.clone());
                self.generation_parity_receipts
                    .insert(next.generation_id_sha256.clone(), receipts);
                if let Some(previous_generation_id) = previous_generation_id
                    && previous_generation_id != next.generation_id_sha256
                {
                    self.generation_parity_receipts
                        .remove(&previous_generation_id);
                }
            }
        }
        let live_generation_ids = self
            .generations
            .values()
            .map(|generation| generation.generation_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        self.generation_parity_receipts
            .retain(|generation_id, _| live_generation_ids.contains(generation_id.as_str()));
    }

    fn classify_semantic_frame(
        &self,
        cohort: &DerivedWinnerCohort,
        frame: &crate::RelationFrame,
        claimed_positive: bool,
    ) -> (SemanticEvidenceOutcome, &'static str) {
        let frame_signature = crate::teacher_program_signature(frame);
        let member_action_matches = semantic_member_action_matches(
            &cohort.members,
            &cohort.law_signature_sha256,
            frame_signature.as_deref(),
            frame,
        );
        let Some(parity) = self.support_parity_case(&frame.frame_id_sha256) else {
            return (
                SemanticEvidenceOutcome::CensoredUnknown,
                "runtime_parity_receipt_missing",
            );
        };
        let execution = crate::execute_response(
            &cohort.winner.program,
            &parity.request_text,
            &parity.provider_payload,
        );
        let parity_matches = execution.status == crate::ResponseExecutionStatus::Executed
            && execution.response.as_deref().is_some_and(|actual| {
                actual == parity.expected_response
                    || crate::online_admission::responses_match_after_execution_budget_normalization(
                        actual,
                        &parity.expected_response,
                    )
            });
        if member_action_matches && parity_matches {
            return (
                SemanticEvidenceOutcome::VerifiedEquivalent,
                "member_law_runtime_parity_pass",
            );
        }
        if member_action_matches {
            return (
                SemanticEvidenceOutcome::HardContradiction,
                "member_law_runtime_parity_mismatch",
            );
        }
        let same_effect_law = crate::teacher_semantic_law_signature(frame).as_deref()
            == Some(cohort.law_signature_sha256.as_str());
        // A positive label belongs to the physical adapter pool. It becomes a
        // contradiction for this cohort only when the effect law also matches.
        if claimed_positive && same_effect_law {
            return (
                SemanticEvidenceOutcome::HardContradiction,
                "claimed_positive_outside_member_law",
            );
        }
        if parity_matches {
            return (
                SemanticEvidenceOutcome::HardContradiction,
                "runtime_parity_pass_without_member_law",
            );
        }
        (
            SemanticEvidenceOutcome::ApplicabilityNegative,
            "verified_cross_law_action",
        )
    }

    fn classified_cohort_pool(
        &self,
        cohort: &DerivedWinnerCohort,
        extra_positive_frames: &[crate::RelationFrame],
    ) -> Option<ClassifiedCohortPool> {
        let winner = &cohort.winner;
        let mut evidence = BTreeMap::<String, ClassifiedSemanticEvidence>::new();
        let mut invariants = Vec::new();
        let mut record = |frame: crate::RelationFrame, claimed_positive: bool| {
            let (outcome, reason) = self.classify_semantic_frame(cohort, &frame, claimed_positive);
            let frame_id = frame.frame_id_sha256.clone();
            let next = ClassifiedSemanticEvidence {
                frame,
                outcome,
                reason,
            };
            evidence
                .entry(frame_id)
                .and_modify(|current| {
                    if semantic_outcome_precedence(next.outcome)
                        > semantic_outcome_precedence(current.outcome)
                    {
                        current.outcome = next.outcome;
                        current.reason = next.reason;
                    }
                })
                .or_insert(next);
        };
        for signature in &cohort.member_signatures {
            let pool = self.pool_snapshot_with_parity(signature)?;
            invariants.extend(pool.invariants);
            for frame in pool.positives {
                record(frame, true);
            }
            for frame in pool.negatives {
                record(frame, false);
            }
        }
        for frame in cohort
            .members
            .iter()
            .filter_map(|member| self.cegis.cohort_evidence(&member.cohort_id_sha256))
            .flat_map(|(_, negatives)| negatives)
        {
            record(frame, false);
        }
        for frame in extra_positive_frames {
            record(frame.clone(), true);
        }
        let mut positives = evidence
            .values()
            .filter(|row| row.outcome == SemanticEvidenceOutcome::VerifiedEquivalent)
            .map(|row| row.frame.clone())
            .collect::<Vec<_>>();
        let mut negatives = evidence
            .values()
            .filter(|row| row.outcome == SemanticEvidenceOutcome::ApplicabilityNegative)
            .map(|row| {
                let mut frame = row.frame.clone();
                frame.verifier_label = Some(false);
                frame
            })
            .collect::<Vec<_>>();
        positives.sort_by(|left, right| {
            left.observed_at_unix_nanos
                .cmp(&right.observed_at_unix_nanos)
                .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
        });
        negatives.sort_by(|left, right| {
            left.observed_at_unix_nanos
                .cmp(&right.observed_at_unix_nanos)
                .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
        });
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
        let pool = TeacherPoolSnapshot {
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
        };
        Some(ClassifiedCohortPool {
            pool,
            evidence: evidence.into_values().collect(),
        })
    }

    fn cohort_pool_snapshot(&self, cohort: &DerivedWinnerCohort) -> Option<TeacherPoolSnapshot> {
        self.classified_cohort_pool(cohort, &[])
            .map(|classified| classified.pool)
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

#[cfg(test)]
#[path = "online_state_tests.rs"]
mod tests;
