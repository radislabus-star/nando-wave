use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::online_subcenter::OnlineSubcenterDiscovery;
use crate::teacher_join::{action_schema_enriched_frame, teacher_actions_have_compatible_effect};
use crate::{
    RelationFrame, RuntimeParityCase, TeacherTransition, relation_frame_online_routing_atom_ids,
    relation_frame_structural_family_id, teacher_action_symbol, teacher_program_signature,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FamilyDiscoveryConfig {
    pub max_teacher_pools: usize,
    pub positive_reservoir_rows: usize,
    pub negative_reservoir_rows: usize,
    pub minimum_invariant_rows: u32,
    pub max_invariants_per_pool: usize,
}

impl Default for FamilyDiscoveryConfig {
    fn default() -> Self {
        Self {
            max_teacher_pools: 256,
            positive_reservoir_rows: 256,
            negative_reservoir_rows: 128,
            minimum_invariant_rows: 16,
            max_invariants_per_pool: 256,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeInvariant {
    pub atom_ids: Vec<u64>,
    pub positive_rows: u32,
    pub positive_tokens: u64,
    pub negative_collisions: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TeacherProgramPool {
    teacher_signature_sha256: String,
    action_symbol: String,
    positives: VecDeque<RelationFrame>,
    positive_rows: u64,
    positive_tokens: u64,
    distinct_surfaces: BTreeSet<u64>,
    distinct_sessions: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GlobalTrainingExample {
    teacher_signature_sha256: String,
    accepted: bool,
    frame: RelationFrame,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TeacherPoolSnapshot {
    pub teacher_signature_sha256: String,
    pub action_symbol: String,
    pub positives: Vec<RelationFrame>,
    pub negatives: Vec<RelationFrame>,
    pub positive_rows: u64,
    pub negative_rows: u64,
    pub positive_tokens: u64,
    pub negative_tokens: u64,
    pub distinct_surfaces: usize,
    pub distinct_sessions: usize,
    pub invariants: Vec<RuntimeInvariant>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct TeacherPoolReport {
    pub teacher_signature_sha256: String,
    pub action_symbol: String,
    pub positive_rows: u64,
    pub positive_tokens: u64,
    pub distinct_surfaces: usize,
    pub distinct_sessions: usize,
    pub retained_positive_rows: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FamilyDiscoveryReport {
    pub transitions_seen: u64,
    pub accepted_transitions: u64,
    pub rejected_transitions: u64,
    pub teacher_pool_count: usize,
    pub cross_program_negative_updates: u64,
    pub same_action_program_negative_updates: u64,
    pub pool_capacity_evictions: u64,
    pub duplicate_rows: u64,
    pub invariant_candidates: usize,
    pub warm_bytes_estimate: usize,
    #[serde(default)]
    pub teacher_pools: Vec<TeacherPoolReport>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CrossSurfaceFamilyDiscovery {
    config: FamilyDiscoveryConfig,
    pools: BTreeMap<String, TeacherProgramPool>,
    #[serde(default)]
    action_pool_counts: BTreeMap<String, usize>,
    subcenters: OnlineSubcenterDiscovery,
    #[serde(default)]
    global_examples: VecDeque<GlobalTrainingExample>,
    #[serde(default)]
    total_tokens: u64,
    seen_events: BTreeMap<String, String>,
    report: FamilyDiscoveryReport,
}

impl CrossSurfaceFamilyDiscovery {
    #[must_use]
    pub fn new(config: FamilyDiscoveryConfig) -> Self {
        Self {
            config,
            pools: BTreeMap::new(),
            action_pool_counts: BTreeMap::new(),
            subcenters: OnlineSubcenterDiscovery::default(),
            global_examples: VecDeque::new(),
            total_tokens: 0,
            seen_events: BTreeMap::new(),
            report: FamilyDiscoveryReport::default(),
        }
    }

    pub fn enforce_runtime_limits(&mut self, config: FamilyDiscoveryConfig) {
        self.config = config;
        while self.pools.len() > self.config.max_teacher_pools {
            self.make_capacity_for_pool();
        }
        for pool in self.pools.values_mut() {
            while pool.positives.len() > self.config.positive_reservoir_rows {
                pool.positives.pop_front();
            }
        }
        let global_limit = self.config.negative_reservoir_rows.saturating_mul(4);
        while self.global_examples.len() > global_limit {
            self.global_examples.pop_front();
        }
        self.rebuild_seen_frames();
        self.action_pool_counts.clear();
        for pool in self.pools.values() {
            *self
                .action_pool_counts
                .entry(pool.action_symbol.clone())
                .or_default() += 1;
        }
        self.refresh_report();
    }

    /// Upgrades bounded historical examples from their independently captured
    /// parity cases before recomputing teacher identities.
    pub fn enrich_action_schemas(
        &mut self,
        parity_cases: &BTreeMap<String, RuntimeParityCase>,
    ) -> usize {
        let mut changed = 0;
        for frame in self
            .pools
            .values_mut()
            .flat_map(|pool| pool.positives.iter_mut())
            .chain(
                self.global_examples
                    .iter_mut()
                    .map(|example| &mut example.frame),
            )
        {
            let enriched =
                action_schema_enriched_frame(frame, parity_cases.get(&frame.frame_id_sha256));
            if enriched != *frame {
                *frame = enriched;
                changed += 1;
            }
        }
        if changed > 0 {
            self.rebuild_seen_frames();
        }
        changed
    }

    /// Recomputes teacher identities with the current action canonicalizer and
    /// merges pools that now describe the same executable program. Historical
    /// row/token totals are preserved when an old pool remains homogeneous.
    pub fn recanonicalize_teacher_signatures(&mut self) -> Result<usize, String> {
        let old_pool_count = self.pools.len();
        let old_pools = std::mem::take(&mut self.pools);
        let mut merged = BTreeMap::<String, TeacherProgramPool>::new();
        for pool in old_pools.into_values() {
            let mut groups = BTreeMap::<String, Vec<RelationFrame>>::new();
            for frame in pool.positives {
                let signature = teacher_program_signature(&frame)
                    .ok_or_else(|| "teacher_signature_migration_missing_action".to_owned())?;
                groups.entry(signature).or_default().push(frame);
            }
            let homogeneous = groups.len() == 1;
            for (signature, frames) in groups {
                let Some(first) = frames.first() else {
                    continue;
                };
                let action_symbol = teacher_action_symbol(first);
                let positive_rows = if homogeneous {
                    pool.positive_rows
                } else {
                    u64::try_from(frames.len()).unwrap_or(u64::MAX)
                };
                let positive_tokens = if homogeneous {
                    pool.positive_tokens
                } else {
                    frames
                        .iter()
                        .map(|frame| frame.estimated_input_tokens)
                        .sum()
                };
                let target =
                    merged
                        .entry(signature.clone())
                        .or_insert_with(|| TeacherProgramPool {
                            teacher_signature_sha256: signature,
                            action_symbol,
                            positives: VecDeque::with_capacity(self.config.positive_reservoir_rows),
                            positive_rows: 0,
                            positive_tokens: 0,
                            distinct_surfaces: BTreeSet::new(),
                            distinct_sessions: BTreeSet::new(),
                        });
                target.positive_rows = target.positive_rows.saturating_add(positive_rows);
                target.positive_tokens = target.positive_tokens.saturating_add(positive_tokens);
                if homogeneous {
                    target
                        .distinct_surfaces
                        .extend(pool.distinct_surfaces.iter().copied());
                    target
                        .distinct_sessions
                        .extend(pool.distinct_sessions.iter().cloned());
                }
                for frame in frames {
                    if target
                        .positives
                        .iter()
                        .any(|existing| existing.frame_id_sha256 == frame.frame_id_sha256)
                    {
                        continue;
                    }
                    if let Some(surface) = relation_frame_structural_family_id(&frame) {
                        target.distinct_surfaces.insert(surface);
                    }
                    target
                        .distinct_sessions
                        .insert(frame.session_id_sha256.clone());
                    push_session_diverse(
                        &mut target.positives,
                        frame,
                        self.config.positive_reservoir_rows,
                    );
                }
            }
        }
        self.pools = merged;

        let mut seen_global = BTreeSet::new();
        let mut global_examples = VecDeque::new();
        for mut example in std::mem::take(&mut self.global_examples) {
            let Some(signature) = teacher_program_signature(&example.frame) else {
                continue;
            };
            if !seen_global.insert(example.frame.frame_id_sha256.clone()) {
                continue;
            }
            example.teacher_signature_sha256 = signature;
            push_global_example(
                &mut global_examples,
                example,
                self.config.negative_reservoir_rows.saturating_mul(4),
            );
        }
        self.global_examples = global_examples;

        self.action_pool_counts.clear();
        self.subcenters = OnlineSubcenterDiscovery::default();
        for pool in self.pools.values() {
            *self
                .action_pool_counts
                .entry(pool.action_symbol.clone())
                .or_default() += 1;
            for frame in &pool.positives {
                self.subcenters.observe(
                    &pool.teacher_signature_sha256,
                    &relation_frame_online_routing_atom_ids(frame),
                    frame.estimated_input_tokens,
                );
            }
        }
        self.report.cross_program_negative_updates = 0;
        self.report.same_action_program_negative_updates = 0;
        self.rebuild_seen_frames();
        self.refresh_report();
        Ok(old_pool_count.saturating_sub(self.pools.len()))
    }

    pub fn observe_transition(&mut self, transition: &TeacherTransition) -> Result<bool, String> {
        let training = transition.as_training_relation_frame();
        let digest = crate::relation_frame_learning_digest(&training)
            .map_err(|error| format!("family_discovery_frame_digest:{error}"))?;
        if let Some(existing) = self.seen_events.get(&training.frame_id_sha256) {
            if existing == &digest {
                self.report.duplicate_rows = self.report.duplicate_rows.saturating_add(1);
                return Ok(false);
            }
            return Err("family_discovery_frame_content_conflict".to_owned());
        }
        self.seen_events
            .insert(training.frame_id_sha256.clone(), digest);
        trim_seen_events(
            &mut self.seen_events,
            self.config.positive_reservoir_rows.saturating_mul(32),
        );
        self.report.transitions_seen = self.report.transitions_seen.saturating_add(1);
        let signature = transition.outcome.action.signature_sha256.as_str();
        let action_symbol = transition.outcome.action.action_symbol.as_str();
        if !self.pools.contains_key(signature) {
            self.make_capacity_for_pool();
            self.pools.insert(
                signature.to_owned(),
                TeacherProgramPool {
                    teacher_signature_sha256: signature.to_owned(),
                    action_symbol: action_symbol.to_owned(),
                    positives: VecDeque::with_capacity(self.config.positive_reservoir_rows),
                    positive_rows: 0,
                    positive_tokens: 0,
                    distinct_surfaces: BTreeSet::new(),
                    distinct_sessions: BTreeSet::new(),
                },
            );
            *self
                .action_pool_counts
                .entry(action_symbol.to_owned())
                .or_default() += 1;
        }

        let accepted = transition.outcome.verifier.accepted;
        self.total_tokens = self
            .total_tokens
            .saturating_add(training.estimated_input_tokens);
        if accepted {
            self.report.accepted_transitions = self.report.accepted_transitions.saturating_add(1);
            let runtime = transition.before.as_routing_relation_frame();
            let atom_ids = relation_frame_online_routing_atom_ids(&runtime);
            self.subcenters
                .observe(signature, &atom_ids, training.estimated_input_tokens);
        } else {
            self.report.rejected_transitions = self.report.rejected_transitions.saturating_add(1);
        }

        if accepted {
            let pool = self.pools.get_mut(signature).expect("teacher pool exists");
            pool.positive_rows = pool.positive_rows.saturating_add(1);
            pool.positive_tokens = pool
                .positive_tokens
                .saturating_add(training.estimated_input_tokens);
            pool.distinct_sessions
                .insert(training.session_id_sha256.clone());
            if let Some(surface) = relation_frame_structural_family_id(&training) {
                pool.distinct_surfaces.insert(surface);
            }
            push_session_diverse(
                &mut pool.positives,
                training.clone(),
                self.config.positive_reservoir_rows,
            );
        }
        let competing_pools = self.pools.len().saturating_sub(1);
        self.report.cross_program_negative_updates = self
            .report
            .cross_program_negative_updates
            .saturating_add(u64::try_from(competing_pools).unwrap_or(u64::MAX));
        let same_action_pools = self
            .action_pool_counts
            .get(action_symbol)
            .copied()
            .unwrap_or(0)
            .saturating_sub(1);
        self.report.same_action_program_negative_updates = self
            .report
            .same_action_program_negative_updates
            .saturating_add(u64::try_from(same_action_pools).unwrap_or(u64::MAX));
        push_global_example(
            &mut self.global_examples,
            GlobalTrainingExample {
                teacher_signature_sha256: signature.to_owned(),
                accepted,
                frame: training,
            },
            self.config.negative_reservoir_rows.saturating_mul(4),
        );
        self.refresh_report();
        Ok(true)
    }

    #[must_use]
    pub fn pool_snapshot(&self, signature: &str) -> Option<TeacherPoolSnapshot> {
        let pool = self.pools.get(signature)?;
        Some(self.snapshot_pool(pool))
    }

    #[must_use]
    pub fn pool_snapshots(&self) -> Vec<TeacherPoolSnapshot> {
        let mut snapshots = self
            .pools
            .values()
            .map(|pool| self.snapshot_pool(pool))
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| {
            right
                .positive_tokens
                .cmp(&left.positive_tokens)
                .then_with(|| right.positive_rows.cmp(&left.positive_rows))
                .then_with(|| {
                    left.teacher_signature_sha256
                        .cmp(&right.teacher_signature_sha256)
                })
        });
        snapshots
    }

    #[must_use]
    pub fn next_pool_signature(&self, after: Option<&str>) -> Option<String> {
        after
            .and_then(|cursor| {
                self.pools
                    .range((
                        std::ops::Bound::Excluded(cursor.to_owned()),
                        std::ops::Bound::Unbounded,
                    ))
                    .next()
                    .map(|(signature, _)| signature.clone())
            })
            .or_else(|| self.pools.keys().next().cloned())
    }

    fn snapshot_pool(&self, pool: &TeacherProgramPool) -> TeacherPoolSnapshot {
        let positives = chronological_unique(pool.positives.iter().cloned());
        let representative = pool.positives.front();
        let negatives = chronological_unique(
            self.global_examples
                .iter()
                .filter(|example| {
                    !example.accepted
                        || (example.teacher_signature_sha256 != pool.teacher_signature_sha256
                            && !representative.is_some_and(|positive| {
                                teacher_actions_have_compatible_effect(positive, &example.frame)
                            }))
                })
                .map(|example| {
                    let mut frame = example.frame.clone();
                    frame.verifier_label = Some(example.accepted);
                    frame
                }),
        );
        let mut invariants = self
            .subcenters
            .clean_subcenters(
                &pool.teacher_signature_sha256,
                self.config.minimum_invariant_rows,
                self.config.max_invariants_per_pool,
            )
            .into_iter()
            .map(|invariant| RuntimeInvariant {
                atom_ids: invariant.atom_ids,
                positive_rows: invariant.positive_rows,
                positive_tokens: invariant.positive_tokens,
                negative_collisions: 0,
            })
            .collect::<Vec<_>>();
        if invariants.is_empty() {
            invariants.extend(exact_clean_intersection(&positives, &negatives));
        }
        minimize_invariant_antichain(&mut invariants);
        TeacherPoolSnapshot {
            teacher_signature_sha256: pool.teacher_signature_sha256.clone(),
            action_symbol: pool.action_symbol.clone(),
            positives,
            negatives,
            positive_rows: pool.positive_rows,
            negative_rows: self
                .report
                .transitions_seen
                .saturating_sub(pool.positive_rows),
            positive_tokens: pool.positive_tokens,
            negative_tokens: self.total_tokens.saturating_sub(pool.positive_tokens),
            distinct_surfaces: pool.distinct_surfaces.len(),
            distinct_sessions: pool.distinct_sessions.len(),
            invariants,
        }
    }

    #[must_use]
    pub fn report(&self) -> FamilyDiscoveryReport {
        let mut report = self.report.clone();
        report.teacher_pool_count = self.pools.len();
        report.invariant_candidates = self
            .pool_snapshots()
            .iter()
            .map(|pool| pool.invariants.len())
            .sum();
        report.warm_bytes_estimate = self.bytes_estimate();
        report.teacher_pools = self
            .pools
            .values()
            .map(|pool| TeacherPoolReport {
                teacher_signature_sha256: pool.teacher_signature_sha256.clone(),
                action_symbol: pool.action_symbol.clone(),
                positive_rows: pool.positive_rows,
                positive_tokens: pool.positive_tokens,
                distinct_surfaces: pool.distinct_surfaces.len(),
                distinct_sessions: pool.distinct_sessions.len(),
                retained_positive_rows: pool.positives.len(),
            })
            .collect();
        report.teacher_pools.sort_by(|left, right| {
            right
                .positive_tokens
                .cmp(&left.positive_tokens)
                .then_with(|| right.positive_rows.cmp(&left.positive_rows))
                .then_with(|| {
                    left.teacher_signature_sha256
                        .cmp(&right.teacher_signature_sha256)
                })
        });
        report
    }

    #[must_use]
    pub fn teacher_pool_count(&self) -> usize {
        self.pools.len()
    }

    fn make_capacity_for_pool(&mut self) {
        if self.pools.len() < self.config.max_teacher_pools {
            return;
        }
        let Some(evict) = self
            .pools
            .iter()
            .min_by_key(|(signature, pool)| {
                (
                    pool.positive_tokens,
                    pool.positive_rows,
                    std::cmp::Reverse((*signature).clone()),
                )
            })
            .map(|(signature, _)| signature.clone())
        else {
            return;
        };
        if let Some(evicted) = self.pools.remove(&evict) {
            let remove_action = self
                .action_pool_counts
                .get_mut(&evicted.action_symbol)
                .is_some_and(|count| {
                    *count = count.saturating_sub(1);
                    *count == 0
                });
            if remove_action {
                self.action_pool_counts.remove(&evicted.action_symbol);
            }
        }
        self.report.pool_capacity_evictions = self.report.pool_capacity_evictions.saturating_add(1);
    }

    fn refresh_report(&mut self) {
        self.report.teacher_pool_count = self.pools.len();
        self.report.warm_bytes_estimate = self.bytes_estimate();
    }

    fn rebuild_seen_frames(&mut self) {
        self.seen_events.clear();
        for frame in self
            .global_examples
            .iter()
            .map(|example| &example.frame)
            .chain(self.pools.values().flat_map(|pool| pool.positives.iter()))
        {
            let Ok(digest) = crate::relation_frame_learning_digest(frame) else {
                continue;
            };
            self.seen_events
                .entry(frame.frame_id_sha256.clone())
                .or_insert(digest);
        }
        trim_seen_events(
            &mut self.seen_events,
            self.config.positive_reservoir_rows.saturating_mul(32),
        );
    }

    fn bytes_estimate(&self) -> usize {
        let frames = self
            .pools
            .values()
            .map(|pool| pool.positives.len())
            .sum::<usize>();
        self.pools
            .len()
            .saturating_mul(512)
            .saturating_add(frames.saturating_mul(1_024))
            .saturating_add(self.global_examples.len().saturating_mul(1_088))
            .saturating_add(self.subcenters.bytes_estimate())
            .saturating_add(self.seen_events.len().saturating_mul(144))
    }
}

impl Default for CrossSurfaceFamilyDiscovery {
    fn default() -> Self {
        Self::new(FamilyDiscoveryConfig::default())
    }
}

fn exact_clean_intersection(
    positives: &[RelationFrame],
    negatives: &[RelationFrame],
) -> Vec<RuntimeInvariant> {
    let Some(first) = positives.first() else {
        return Vec::new();
    };
    let mut common = relation_frame_online_routing_atom_ids(first)
        .into_iter()
        .collect::<BTreeSet<_>>();
    for positive in &positives[1..] {
        let atoms = relation_frame_online_routing_atom_ids(positive)
            .into_iter()
            .collect::<BTreeSet<_>>();
        common.retain(|atom| atoms.contains(atom));
    }
    common
        .into_iter()
        .filter(|atom| {
            negatives.iter().all(|negative| {
                relation_frame_online_routing_atom_ids(negative)
                    .binary_search(atom)
                    .is_err()
            })
        })
        .map(|atom| RuntimeInvariant {
            atom_ids: vec![atom],
            positive_rows: u32::try_from(positives.len()).unwrap_or(u32::MAX),
            positive_tokens: positives
                .iter()
                .map(|frame| frame.estimated_input_tokens)
                .sum(),
            negative_collisions: 0,
        })
        .collect()
}

fn minimize_invariant_antichain(invariants: &mut Vec<RuntimeInvariant>) {
    invariants.retain(|invariant| !invariant.atom_ids.is_empty());
    for invariant in invariants.iter_mut() {
        invariant.atom_ids.sort_unstable();
        invariant.atom_ids.dedup();
    }
    invariants.sort_by(|left, right| {
        left.atom_ids
            .len()
            .cmp(&right.atom_ids.len())
            .then_with(|| right.positive_tokens.cmp(&left.positive_tokens))
            .then_with(|| left.atom_ids.cmp(&right.atom_ids))
    });
    let mut minimal = Vec::<RuntimeInvariant>::new();
    for invariant in invariants.drain(..) {
        if minimal.iter().any(|kept| {
            kept.atom_ids
                .iter()
                .all(|atom| invariant.atom_ids.binary_search(atom).is_ok())
        }) {
            continue;
        }
        minimal.push(invariant);
    }
    *invariants = minimal;
}

fn chronological_unique(frames: impl IntoIterator<Item = RelationFrame>) -> Vec<RelationFrame> {
    let mut frames = frames.into_iter().collect::<Vec<_>>();
    frames.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });
    let mut frame_ids = BTreeSet::new();
    frames.retain(|frame| frame_ids.insert(frame.frame_id_sha256.clone()));
    frames
}

fn push_global_example(
    rows: &mut VecDeque<GlobalTrainingExample>,
    row: GlobalTrainingExample,
    limit: usize,
) {
    if limit == 0 {
        return;
    }
    while rows.len() >= limit {
        let mut counts = BTreeMap::<String, usize>::new();
        for existing in rows.iter() {
            *counts
                .entry(existing.teacher_signature_sha256.clone())
                .or_default() += 1;
        }
        let removable = rows.iter().position(|existing| {
            counts
                .get(existing.teacher_signature_sha256.as_str())
                .copied()
                .unwrap_or(0)
                > 1
        });
        if let Some(index) = removable {
            rows.remove(index);
        } else {
            rows.pop_front();
        }
    }
    rows.push_back(row);
}

fn push_session_diverse(rows: &mut VecDeque<RelationFrame>, row: RelationFrame, limit: usize) {
    if limit == 0 {
        return;
    }
    if rows.len() >= limit {
        let repeated_session = rows
            .iter()
            .position(|existing| existing.session_id_sha256 == row.session_id_sha256);
        if let Some(index) = repeated_session {
            rows.remove(index);
        } else {
            rows.pop_front();
        }
    }
    rows.push_back(row);
}

fn trim_seen_events(events: &mut BTreeMap<String, String>, limit: usize) {
    while events.len() > limit.max(1) {
        let Some(key) = events.keys().next().cloned() else {
            break;
        };
        events.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RELATION_FRAME_SCHEMA, RelationAtom, SOURCE_NEUTRAL_EXTRACTOR_VERSION};

    fn wait_frame(id: u64, max_tokens: u64) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: format!("{id:064x}"),
            event_id_sha256: format!("{:064x}", id + 100),
            client_intent_id_sha256: format!("{:064x}", id + 200),
            session_id_sha256: format!("{:064x}", id + 300),
            observed_at_unix_nanos: id,
            estimated_input_tokens: 100,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::TypedSlot {
                    slot_id: 1,
                    value_type: crate::AtomValueType::Identifier,
                    source: crate::AtomSource::Observation,
                    value_sha256: "6".repeat(64),
                },
                RelationAtom::TypedSlot {
                    slot_id: 2,
                    value_type: crate::AtomValueType::Identifier,
                    source: crate::AtomSource::Action,
                    value_sha256: "6".repeat(64),
                },
                RelationAtom::ObservationSelector {
                    slot_id: 1,
                    selector: crate::ResponseValueSelector::ContentLinePrefix {
                        prefix: "Script running with cell ID ".to_owned(),
                        value_type: crate::AtomValueType::Identifier,
                    },
                },
                RelationAtom::UniqueSlot { slot_id: 1 },
                RelationAtom::SlotEquality {
                    left_slot: 1,
                    right_slot: 2,
                },
                RelationAtom::ActionFunction {
                    value: "wait".to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "cell_id".to_owned(),
                    slot_id: 2,
                    value_type: None,
                },
                RelationAtom::ActionIntegerArgument {
                    name: "max_tokens".to_owned(),
                    value: max_tokens,
                },
                RelationAtom::CompletionState {
                    value: "pending".to_owned(),
                },
            ],
            evidence_ref_sha256: format!("{:064x}", id + 400),
        }
    }

    fn custom_continuation_frame(id: u64) -> RelationFrame {
        let mut frame = wait_frame(id, 12_000);
        frame.atoms.retain(|atom| {
            !matches!(
                atom,
                RelationAtom::ActionFunction { .. }
                    | RelationAtom::ActionRoleArgument { .. }
                    | RelationAtom::ActionIntegerArgument { .. }
            )
        });
        frame.atoms.extend([
            RelationAtom::ActionCustomTool {
                value: "exec".to_owned(),
            },
            RelationAtom::ActionInnerTool {
                value: "write_stdin".to_owned(),
            },
            RelationAtom::ActionRoleArgument {
                name: "session_id".to_owned(),
                slot_id: 2,
                value_type: None,
            },
            RelationAtom::ActionStringArgument {
                name: "chars".to_owned(),
                value: String::new(),
            },
        ]);
        frame
    }

    #[test]
    fn compatible_continuation_transport_is_not_a_cross_program_negative() {
        let wait = wait_frame(10, 12_000);
        let custom = custom_continuation_frame(20);
        let mut terminate = wait_frame(30, 12_000);
        terminate.atoms.push(RelationAtom::ActionBooleanArgument {
            name: "terminate".to_owned(),
            value: true,
        });
        let wait_signature = teacher_program_signature(&wait).expect("wait signature");
        let mut discovery = CrossSurfaceFamilyDiscovery::default();
        for frame in [&wait, &custom, &terminate] {
            let transition =
                crate::teacher_transition_from_completed(frame, None).expect("teacher transition");
            assert_eq!(discovery.observe_transition(&transition), Ok(true));
        }

        let snapshot = discovery.pool_snapshot(&wait_signature).expect("wait pool");
        assert_eq!(snapshot.negatives.len(), 1);
        assert!(snapshot.negatives[0].atoms.iter().any(
            |atom| matches!(atom, RelationAtom::ActionBooleanArgument { name, value: true } if name == "terminate")
        ));
        assert!(
            !snapshot.negatives[0]
                .atoms
                .iter()
                .any(|atom| matches!(atom, RelationAtom::ActionCustomTool { .. }))
        );
    }

    #[test]
    fn signature_migration_merges_budget_variants_and_removes_false_negatives() {
        let first = wait_frame(1, 1_000);
        let second = wait_frame(2, 5_000);
        let mut discovery = CrossSurfaceFamilyDiscovery::default();
        discovery.pools.insert(
            "old-a".to_owned(),
            TeacherProgramPool {
                teacher_signature_sha256: "old-a".to_owned(),
                action_symbol: "function:wait".to_owned(),
                positives: VecDeque::from([first.clone()]),
                positive_rows: 12,
                positive_tokens: 1_200,
                distinct_surfaces: BTreeSet::new(),
                distinct_sessions: BTreeSet::from([first.session_id_sha256.clone()]),
            },
        );
        discovery.pools.insert(
            "old-b".to_owned(),
            TeacherProgramPool {
                teacher_signature_sha256: "old-b".to_owned(),
                action_symbol: "function:wait".to_owned(),
                positives: VecDeque::from([second.clone()]),
                positive_rows: 7,
                positive_tokens: 700,
                distinct_surfaces: BTreeSet::new(),
                distinct_sessions: BTreeSet::from([second.session_id_sha256.clone()]),
            },
        );
        discovery.global_examples = VecDeque::from([
            GlobalTrainingExample {
                teacher_signature_sha256: "old-a".to_owned(),
                accepted: true,
                frame: first,
            },
            GlobalTrainingExample {
                teacher_signature_sha256: "old-b".to_owned(),
                accepted: true,
                frame: second,
            },
        ]);

        assert_eq!(discovery.recanonicalize_teacher_signatures(), Ok(1));
        let snapshots = discovery.pool_snapshots();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].positive_rows, 19);
        assert_eq!(snapshots[0].positive_tokens, 1_900);
        assert!(snapshots[0].negatives.is_empty());
        assert_eq!(snapshots[0].positives.len(), 2);
    }
}
