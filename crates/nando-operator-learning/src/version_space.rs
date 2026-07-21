use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{
    PhaseCenterCell, add_phase_vector, phase_center_from_sum, phase_coherence,
    phase_vector_from_atom_ids,
};
use serde::{Deserialize, Serialize};

use crate::{
    CollectionOutputRenderer, RelationFrame, ResponseOperation, ResponseProgram,
    relation_frame_online_routing_atom_ids, response_program_required_routing_atom_ids,
};

pub type AstNodeId = u32;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VersionSpaceConfig {
    pub max_ast_nodes: usize,
    pub max_depth: u8,
    pub max_complete_candidates: usize,
    pub exact_checks_per_slice: usize,
}

impl Default for VersionSpaceConfig {
    fn default() -> Self {
        Self {
            max_ast_nodes: 100_000,
            max_depth: 4,
            max_complete_candidates: 4_096,
            exact_checks_per_slice: 32,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AstProgramKind {
    PlanAdvance,
    FunctionCall,
    CustomToolCall,
    Project,
    Status,
    Collection,
    Legacy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InternedProgram {
    pub node_id: AstNodeId,
    pub digest_sha256: String,
    pub kind: AstProgramKind,
    pub depth: u8,
    pub serialized_bytes: usize,
    pub phase_score_micro: i64,
    pub program: ResponseProgram,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct VersionSpaceReport {
    pub ast_nodes: usize,
    pub survivors: usize,
    pub eliminated: usize,
    pub frontier_cursor: usize,
    pub exact_checks: u64,
    pub slices_completed: u64,
    pub capacity_rejections: u64,
    pub depth_rejections: u64,
    pub duplicate_programs: u64,
    pub maximum_depth_seen: u8,
    pub serialized_bytes: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VersionSpaceArena {
    config: VersionSpaceConfig,
    nodes: Vec<InternedProgram>,
    index: BTreeMap<String, AstNodeId>,
    ranked_frontier: Vec<AstNodeId>,
    survivors: BTreeSet<AstNodeId>,
    eliminated: BTreeMap<AstNodeId, String>,
    frontier_cursor: usize,
    exact_checks: u64,
    slices_completed: u64,
    capacity_rejections: u64,
    depth_rejections: u64,
    duplicate_programs: u64,
    maximum_depth_seen: u8,
}

impl VersionSpaceArena {
    #[must_use]
    pub fn new(config: VersionSpaceConfig) -> Self {
        Self {
            config,
            nodes: Vec::new(),
            index: BTreeMap::new(),
            ranked_frontier: Vec::new(),
            survivors: BTreeSet::new(),
            eliminated: BTreeMap::new(),
            frontier_cursor: 0,
            exact_checks: 0,
            slices_completed: 0,
            capacity_rejections: 0,
            depth_rejections: 0,
            duplicate_programs: 0,
            maximum_depth_seen: 0,
        }
    }

    pub fn intern(&mut self, program: ResponseProgram) -> Option<AstNodeId> {
        let depth = response_program_depth(&program);
        self.maximum_depth_seen = self.maximum_depth_seen.max(depth);
        if depth > self.config.max_depth {
            self.depth_rejections = self.depth_rejections.saturating_add(1);
            return None;
        }
        let bytes = serde_json::to_vec(&program).ok()?;
        let digest_sha256 = crate::sha256_bytes(&bytes);
        if let Some(node_id) = self.index.get(&digest_sha256) {
            self.duplicate_programs = self.duplicate_programs.saturating_add(1);
            return Some(*node_id);
        }
        if self.nodes.len() >= self.config.max_ast_nodes {
            self.capacity_rejections = self.capacity_rejections.saturating_add(1);
            return None;
        }
        let node_id = u32::try_from(self.nodes.len()).ok()?;
        self.nodes.push(InternedProgram {
            node_id,
            digest_sha256: digest_sha256.clone(),
            kind: response_program_kind(&program),
            depth,
            serialized_bytes: bytes.len(),
            phase_score_micro: 0,
            program,
        });
        self.index.insert(digest_sha256, node_id);
        self.survivors.insert(node_id);
        Some(node_id)
    }

    pub fn intern_all<I>(&mut self, programs: I) -> usize
    where
        I: IntoIterator<Item = ResponseProgram>,
    {
        programs
            .into_iter()
            .filter_map(|program| self.intern(program))
            .count()
    }

    pub fn rank_for_support(&mut self, support: &[RelationFrame]) {
        self.rank_for_phase_centers(support, &[]);
    }

    pub fn rank_for_phase_centers(
        &mut self,
        positives: &[RelationFrame],
        negatives: &[RelationFrame],
    ) {
        let positive_center = phase_center_for_frames(positives);
        let negative_center = phase_center_for_frames(negatives);
        for node in &mut self.nodes {
            let program_vector = phase_vector_from_atom_ids(
                response_program_required_routing_atom_ids(&node.program),
                16,
            );
            let score = if negatives.is_empty() {
                positives
                    .iter()
                    .map(|frame| {
                        phase_coherence(
                            &phase_vector_from_atom_ids(
                                relation_frame_online_routing_atom_ids(frame),
                                16,
                            ),
                            &program_vector,
                        )
                    })
                    .sum::<f64>()
            } else {
                phase_coherence(&program_vector, &positive_center)
                    - phase_coherence(&program_vector, &negative_center)
            };
            node.phase_score_micro = finite_micro(score);
        }
        let mut frontier = self.survivors.iter().copied().collect::<Vec<AstNodeId>>();
        frontier.sort_by(|left, right| {
            let left = &self.nodes[usize::try_from(*left).unwrap_or(usize::MAX)];
            let right = &self.nodes[usize::try_from(*right).unwrap_or(usize::MAX)];
            right
                .phase_score_micro
                .cmp(&left.phase_score_micro)
                .then_with(|| left.serialized_bytes.cmp(&right.serialized_bytes))
                .then_with(|| left.depth.cmp(&right.depth))
                .then_with(|| left.digest_sha256.cmp(&right.digest_sha256))
        });
        frontier.truncate(self.config.max_complete_candidates);
        self.ranked_frontier = frontier;
        self.frontier_cursor = 0;
    }

    #[must_use]
    pub fn next_slice(&mut self) -> Vec<InternedProgram> {
        if self.frontier_cursor >= self.ranked_frontier.len() {
            return Vec::new();
        }
        let end = self
            .frontier_cursor
            .saturating_add(self.config.exact_checks_per_slice)
            .min(self.ranked_frontier.len());
        let slice = self.ranked_frontier[self.frontier_cursor..end]
            .iter()
            .filter(|node_id| self.survivors.contains(node_id))
            .filter_map(|node_id| {
                usize::try_from(*node_id)
                    .ok()
                    .and_then(|index| self.nodes.get(index))
                    .cloned()
            })
            .collect::<Vec<_>>();
        self.frontier_cursor = end;
        self.slices_completed = self.slices_completed.saturating_add(1);
        slice
    }

    pub fn next_candidate(&mut self) -> Option<InternedProgram> {
        while self.frontier_cursor < self.ranked_frontier.len() {
            let node_id = self.ranked_frontier[self.frontier_cursor];
            self.frontier_cursor = self.frontier_cursor.saturating_add(1);
            if self.survivors.contains(&node_id) {
                return usize::try_from(node_id)
                    .ok()
                    .and_then(|index| self.nodes.get(index))
                    .cloned();
            }
        }
        None
    }

    #[must_use]
    pub const fn exact_checks_per_slice(&self) -> usize {
        self.config.exact_checks_per_slice
    }

    pub fn begin_slice(&mut self) {
        self.slices_completed = self.slices_completed.saturating_add(1);
    }

    #[must_use]
    pub fn has_pending_candidates(&self) -> bool {
        self.frontier_cursor < self.ranked_frontier.len()
            && self.ranked_frontier[self.frontier_cursor..]
                .iter()
                .any(|node_id| self.survivors.contains(node_id))
    }

    #[must_use]
    pub fn phase_rank(&self, node_id: AstNodeId) -> Option<u32> {
        self.ranked_frontier
            .iter()
            .position(|candidate| *candidate == node_id)
            .and_then(|rank| u32::try_from(rank.saturating_add(1)).ok())
    }

    pub fn record_exact_check(&mut self, node_id: AstNodeId, accepted: bool, reason: &str) {
        self.exact_checks = self.exact_checks.saturating_add(1);
        if accepted {
            return;
        }
        self.survivors.remove(&node_id);
        self.eliminated
            .entry(node_id)
            .or_insert_with(|| reason.to_owned());
    }

    pub fn reset_frontier(&mut self) {
        self.frontier_cursor = 0;
        self.ranked_frontier
            .retain(|id| self.survivors.contains(id));
    }

    #[must_use]
    pub fn survivor_programs(&self) -> Vec<InternedProgram> {
        self.survivors
            .iter()
            .filter_map(|node_id| {
                usize::try_from(*node_id)
                    .ok()
                    .and_then(|index| self.nodes.get(index))
                    .cloned()
            })
            .collect()
    }

    #[must_use]
    pub fn report(&self) -> VersionSpaceReport {
        VersionSpaceReport {
            ast_nodes: self.nodes.len(),
            survivors: self.survivors.len(),
            eliminated: self.eliminated.len(),
            frontier_cursor: self.frontier_cursor,
            exact_checks: self.exact_checks,
            slices_completed: self.slices_completed,
            capacity_rejections: self.capacity_rejections,
            depth_rejections: self.depth_rejections,
            duplicate_programs: self.duplicate_programs,
            maximum_depth_seen: self.maximum_depth_seen,
            serialized_bytes: self.nodes.iter().map(|node| node.serialized_bytes).sum(),
        }
    }
}

impl Default for VersionSpaceArena {
    fn default() -> Self {
        Self::new(VersionSpaceConfig::default())
    }
}

#[must_use]
pub fn response_program_depth(program: &ResponseProgram) -> u8 {
    match &program.operation {
        ResponseOperation::ComposeCollection {
            steps, renderer, ..
        } => {
            let render_depth = match renderer {
                CollectionOutputRenderer::Direct => 0,
                CollectionOutputRenderer::RenderTemplate { .. } => 1,
                CollectionOutputRenderer::RenderSequence { segments } => {
                    u8::from(!segments.is_empty())
                }
                CollectionOutputRenderer::RequestTemplate { .. } => 1,
            };
            u8::try_from(steps.len())
                .unwrap_or(u8::MAX)
                .max(1)
                .saturating_add(render_depth)
        }
        ResponseOperation::ProjectSelectedValue { renderer, .. } => match renderer {
            CollectionOutputRenderer::Direct => 1,
            CollectionOutputRenderer::RenderTemplate { .. }
            | CollectionOutputRenderer::RenderSequence { .. }
            | CollectionOutputRenderer::RequestTemplate { .. } => 2,
        },
        _ => 1,
    }
}

#[must_use]
pub fn response_program_kind(program: &ResponseProgram) -> AstProgramKind {
    match &program.operation {
        ResponseOperation::UniqueConsensus { variants, .. } => variants
            .first()
            .map(|variant| response_program_kind(&variant.program))
            .unwrap_or(AstProgramKind::Legacy),
        ResponseOperation::AdvancePlan { .. } => AstProgramKind::PlanAdvance,
        ResponseOperation::FunctionCallFromRoles { .. } => AstProgramKind::FunctionCall,
        ResponseOperation::CustomToolCallFromRoles { .. } => AstProgramKind::CustomToolCall,
        ResponseOperation::ProjectSelectedValue { .. } => AstProgramKind::Project,
        ResponseOperation::ProjectStatus { .. } => AstProgramKind::Status,
        ResponseOperation::ComposeCollection { .. } => AstProgramKind::Collection,
        ResponseOperation::CopyAfterPrefix { .. }
        | ResponseOperation::TestResultSummary { .. }
        | ResponseOperation::WaitOnYieldedCell { .. }
        | ResponseOperation::WaitOnAnyYieldedCell { .. }
        | ResponseOperation::WaitOnYieldedSurfaces { .. } => AstProgramKind::Legacy,
    }
}

fn phase_center_for_frames(frames: &[RelationFrame]) -> Vec<PhaseCenterCell> {
    let mut sum = vec![PhaseCenterCell::default(); 16];
    for frame in frames {
        let vector = phase_vector_from_atom_ids(relation_frame_online_routing_atom_ids(frame), 16);
        add_phase_vector(&mut sum, &vector, 1.0);
    }
    phase_center_from_sum(&sum)
}

fn finite_micro(value: f64) -> i64 {
    if !value.is_finite() {
        return i64::MIN;
    }
    let scaled = value * 1_000_000.0;
    if scaled >= i64::MAX as f64 {
        i64::MAX
    } else if scaled <= i64::MIN as f64 {
        i64::MIN
    } else {
        scaled.round() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AtomValueType, ProjectStatusMapping, RELATION_FRAME_SCHEMA, RelationAtom,
        ResponseValueSelector, SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    };

    fn frame(index: u64, atom_ids: &[u64]) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: format!("{index:064x}"),
            event_id_sha256: format!("{:064x}", index.saturating_add(1_000)),
            client_intent_id_sha256: format!("{:064x}", index.saturating_add(2_000)),
            session_id_sha256: format!("{:064x}", index.saturating_add(3_000)),
            observed_at_unix_nanos: index,
            estimated_input_tokens: 100,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: atom_ids
                .iter()
                .map(|atom_id| RelationAtom::ClientCapabilityAtom { atom_id: *atom_id })
                .collect(),
            evidence_ref_sha256: format!("{:064x}", index.saturating_add(4_000)),
        }
    }

    fn exact_checks_until(arena: &mut VersionSpaceArena, winner: AstNodeId) -> u64 {
        while let Some(candidate) = arena.next_candidate() {
            arena.record_exact_check(
                candidate.node_id,
                candidate.node_id == winner,
                "causal_rank_control",
            );
            if candidate.node_id == winner {
                break;
            }
        }
        arena.report().exact_checks
    }

    #[test]
    fn learned_anti_center_changes_top_one_and_halves_exact_search() {
        let programs = vec![
            ResponseProgram::project_status(
                ResponseValueSelector::UniqueScalar {
                    value_type: AtomValueType::Integer,
                },
                ProjectStatusMapping::ZeroIsSuccess,
                "completed",
            ),
            ResponseProgram::project_status(
                ResponseValueSelector::JsonField {
                    field: "status".to_owned(),
                    value_type: AtomValueType::Integer,
                },
                ProjectStatusMapping::ZeroIsSuccess,
                "completed",
            ),
        ];
        let positive_atoms = programs
            .iter()
            .flat_map(response_program_required_routing_atom_ids)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let positives = vec![frame(1, &positive_atoms)];

        let mut positive_only = VersionSpaceArena::default();
        positive_only.intern_all(programs.clone());
        positive_only.rank_for_phase_centers(&positives, &[]);
        let rejected = positive_only
            .next_candidate()
            .expect("positive-only top one");
        let winner = positive_only
            .next_candidate()
            .expect("positive-only second");

        let negative_atoms = response_program_required_routing_atom_ids(&rejected.program);
        let negatives = vec![frame(2, &negative_atoms)];

        let mut no_anti = VersionSpaceArena::default();
        no_anti.intern_all(programs.clone());
        no_anti.rank_for_phase_centers(&positives, &[]);
        let no_anti_checks = exact_checks_until(&mut no_anti, winner.node_id);

        let mut full_phase = VersionSpaceArena::default();
        full_phase.intern_all(programs);
        full_phase.rank_for_phase_centers(&positives, &negatives);
        let full_phase_checks = exact_checks_until(&mut full_phase, winner.node_id);

        assert_eq!(no_anti_checks, 2);
        assert_eq!(full_phase_checks, 1);
        assert!(full_phase_checks < no_anti_checks);
    }
}
