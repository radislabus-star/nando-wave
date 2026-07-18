use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use super::{
    OperatorCircuit, OperatorCircuitError, OperatorCircuitRelation, OperatorRelationCell,
    PhaseCenterCell, TernaryRelationState, TransformOp8,
};

pub const OPERATOR_BLUEPRINT_MAX_BUNDLES: usize = 16;
pub const OPERATOR_BLUEPRINT_MAX_ROLES: usize = 32;
pub const OPERATOR_BLUEPRINT_MAX_RELATIONS: usize = 256;
pub const OPERATOR_BLUEPRINT_MAX_PROGRAM_ATOMS: usize = 16;
pub const OPERATOR_BLUEPRINT_MAX_ALIGNMENTS: usize = 64;
pub const OPERATOR_BLUEPRINT_MAX_BEAM_DEPTH: usize = 12;
pub const OPERATOR_BLUEPRINT_MAX_EXPANSIONS: usize = 4_096;
pub const OPERATOR_ROLE_COLOR_ROUNDS: usize = 3;
pub const OPERATOR_BLUEPRINT_CANONICALIZER_VERSION: u32 = 1;

pub type Commitment256 = [u8; 32];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StructuralRoleSignature {
    type_class: u8,
    cardinality_class: u8,
    temporal_position: u8,
    constraint_mask: u32,
    neighboring_relation_planes: Box<[u8]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalRelationFragment {
    pub plane: u8,
    pub source_local_role: u8,
    pub target_local_role: u8,
    pub state: TernaryRelationState,
    pub phase_anchor: PhaseCenterCell,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TypedProgramAtom {
    pub opcode: u8,
    pub output_local_role: u8,
    pub source_a_local_role: u8,
    pub source_b_local_role: u8,
    pub parameter: u16,
    pub flags: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFragmentBundle {
    lineage_sha256: Commitment256,
    surface_sha256: Commitment256,
    roles: Box<[StructuralRoleSignature]>,
    relations: Box<[LocalRelationFragment]>,
    program_atoms: Box<[TypedProgramAtom]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceFragmentBundleError {
    EmptyLineage,
    EmptySurface,
    EmptyRoles,
    TooManyRoles,
    TooManyRelations,
    TooManyProgramAtoms,
    InvalidLocalRole,
    SelfRelation,
    DuplicateRelation,
    InvalidPhase,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RoleBinding {
    pub bundle_index: u8,
    pub local_role: u8,
    pub canonical_role: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleAlignmentHypothesis {
    bindings: Box<[RoleBinding]>,
    canonical_role_count: u8,
    fingerprint_sha256: Commitment256,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoleAlignmentConfig {
    pub max_hypotheses: usize,
    pub max_expansions: usize,
    pub color_rounds: usize,
}

impl Default for RoleAlignmentConfig {
    fn default() -> Self {
        Self {
            max_hypotheses: OPERATOR_BLUEPRINT_MAX_ALIGNMENTS,
            max_expansions: OPERATOR_BLUEPRINT_MAX_EXPANSIONS,
            color_rounds: OPERATOR_ROLE_COLOR_ROUNDS,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoleAlignmentBlocker {
    TooFewBundles,
    DuplicateLineage,
    InvalidConfig,
    NoCompatibleAlignment,
    RoleCapacityReached,
    BudgetExhausted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleAlignmentReport {
    pub hypotheses: Box<[RoleAlignmentHypothesis]>,
    pub expansions: usize,
    pub symmetric_branches: usize,
    pub complete: bool,
    pub blocker: Option<RoleAlignmentBlocker>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleGraph {
    role_count: u8,
    bindings: Box<[RoleBinding]>,
    canonical_roles: Box<[StructuralRoleSignature]>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CompositionEdge {
    pub producer_step: u8,
    pub consumer_step: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionDag {
    edges: Box<[CompositionEdge]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RendererContract {
    commitment_sha256: Commitment256,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifierContract {
    commitment_sha256: Commitment256,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateOperatorBlueprint {
    role_graph: RoleGraph,
    relation_program: OperatorCircuit,
    transform_program: Box<[TransformOp8]>,
    composition_dag: CompositionDag,
    renderer_hypothesis: RendererContract,
    verifier_contract: VerifierContract,
    fingerprint_sha256: Commitment256,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlueprintBeamConfig {
    pub max_blueprints: usize,
    pub max_depth: usize,
    pub max_expansions: usize,
}

impl Default for BlueprintBeamConfig {
    fn default() -> Self {
        Self {
            max_blueprints: OPERATOR_BLUEPRINT_MAX_ALIGNMENTS,
            max_depth: OPERATOR_BLUEPRINT_MAX_BEAM_DEPTH,
            max_expansions: OPERATOR_BLUEPRINT_MAX_EXPANSIONS,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BlueprintSynthesisBlocker {
    AlignmentIncomplete,
    InvalidConfig,
    EmptyRelations,
    DisconnectedRelations,
    BeamDepthReached,
    ExpansionBudgetReached,
    TransformCapacityReached,
    InvalidCircuit,
    NoBlueprint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlueprintSynthesisBlockerCount {
    pub blocker: BlueprintSynthesisBlocker,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlueprintSynthesisReport {
    pub blueprints: Box<[CandidateOperatorBlueprint]>,
    pub expansions: usize,
    pub complete: bool,
    pub blockers: Box<[BlueprintSynthesisBlockerCount]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrozenOperatorBlueprintSet {
    source_generation: u64,
    canonicalizer_version: u32,
    config: BlueprintBeamConfig,
    support_lineages_sha256: Box<[Commitment256]>,
    candidate_set_sha256: Commitment256,
    blueprints: Box<[CandidateOperatorBlueprint]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrozenBlueprintError {
    IncompleteSynthesis,
    EmptyCandidateSet,
    SupportLineageReused,
    DuplicateFutureLineage,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrozenBlueprintFutureWindow {
    frozen: FrozenOperatorBlueprintSet,
    future_lineages_sha256: BTreeSet<Commitment256>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlueprintFutureConfig {
    pub min_lineages_per_edge: usize,
    pub coherence_floor: f64,
    pub coherence_margin: f64,
}

impl Default for BlueprintFutureConfig {
    fn default() -> Self {
        Self {
            min_lineages_per_edge: 1,
            coherence_floor: 0.90,
            coherence_margin: 0.10,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlueprintPhaseControl {
    Full,
    NoPhase,
    ShuffledPhase,
    MagnitudeOnly,
    MatchedRandomCenter,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlueprintFutureScore {
    pub blueprint_fingerprint_sha256: Commitment256,
    pub edge_coherences: Box<[f64]>,
    pub plane_coherences: Box<[f64]>,
    pub whole_circuit_coherence: f64,
    pub covered_edges: usize,
    pub covered_planes: usize,
    pub ambiguous_bindings: usize,
    pub eligible: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlueprintFutureBlocker {
    InvalidConfig,
    SupportLineageReused,
    DuplicateFutureLineage,
    NoEligibleBlueprint,
    CoherenceBelowFloor,
    InsufficientRunnerUpMargin,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlueprintFutureReport {
    pub control: BlueprintPhaseControl,
    pub scores: Box<[BlueprintFutureScore]>,
    pub winner_fingerprint_sha256: Option<Commitment256>,
    pub runner_up_margin: f64,
    pub blocker: Option<BlueprintFutureBlocker>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StructuralRoleCanonicalizer;

#[derive(Clone, Copy, Debug, Default)]
pub struct BoundedRoleAligner;

#[derive(Clone, Copy, Debug, Default)]
pub struct BoundedCircuitBeam;

#[derive(Clone, Copy, Debug, Default)]
pub struct BlueprintFutureEvaluator;

#[derive(Clone, Debug)]
struct AlignmentState {
    bindings: Vec<RoleBinding>,
    canonical_role_count: u8,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PhaseModeKey {
    state: TernaryRelationState,
    slot: u8,
}

#[derive(Clone, Copy, Debug, Default)]
struct PhaseModeAggregate {
    re: f64,
    im: f64,
}

impl StructuralRoleSignature {
    #[must_use]
    pub fn new(
        type_class: u8,
        cardinality_class: u8,
        temporal_position: u8,
        constraint_mask: u32,
        mut neighboring_relation_planes: Vec<u8>,
    ) -> Self {
        neighboring_relation_planes.sort_unstable();
        neighboring_relation_planes.dedup();
        Self {
            type_class,
            cardinality_class,
            temporal_position,
            constraint_mask,
            neighboring_relation_planes: neighboring_relation_planes.into_boxed_slice(),
        }
    }

    #[must_use]
    pub const fn type_class(&self) -> u8 {
        self.type_class
    }

    #[must_use]
    pub const fn cardinality_class(&self) -> u8 {
        self.cardinality_class
    }

    #[must_use]
    pub const fn temporal_position(&self) -> u8 {
        self.temporal_position
    }

    #[must_use]
    pub const fn constraint_mask(&self) -> u32 {
        self.constraint_mask
    }

    #[must_use]
    pub fn neighboring_relation_planes(&self) -> &[u8] {
        &self.neighboring_relation_planes
    }
}

impl SurfaceFragmentBundle {
    pub fn new(
        lineage_sha256: Commitment256,
        surface_sha256: Commitment256,
        roles: Vec<StructuralRoleSignature>,
        mut relations: Vec<LocalRelationFragment>,
        mut program_atoms: Vec<TypedProgramAtom>,
    ) -> Result<Self, SurfaceFragmentBundleError> {
        if lineage_sha256 == [0; 32] {
            return Err(SurfaceFragmentBundleError::EmptyLineage);
        }
        if surface_sha256 == [0; 32] {
            return Err(SurfaceFragmentBundleError::EmptySurface);
        }
        if roles.is_empty() {
            return Err(SurfaceFragmentBundleError::EmptyRoles);
        }
        if roles.len() > OPERATOR_BLUEPRINT_MAX_ROLES {
            return Err(SurfaceFragmentBundleError::TooManyRoles);
        }
        if relations.len() > OPERATOR_BLUEPRINT_MAX_RELATIONS {
            return Err(SurfaceFragmentBundleError::TooManyRelations);
        }
        if program_atoms.len() > OPERATOR_BLUEPRINT_MAX_PROGRAM_ATOMS {
            return Err(SurfaceFragmentBundleError::TooManyProgramAtoms);
        }
        for relation in &relations {
            if usize::from(relation.source_local_role) >= roles.len()
                || usize::from(relation.target_local_role) >= roles.len()
            {
                return Err(SurfaceFragmentBundleError::InvalidLocalRole);
            }
            if relation.source_local_role == relation.target_local_role {
                return Err(SurfaceFragmentBundleError::SelfRelation);
            }
            let magnitude = relation.phase_anchor.re.hypot(relation.phase_anchor.im);
            if !magnitude.is_finite() || magnitude <= f64::EPSILON {
                return Err(SurfaceFragmentBundleError::InvalidPhase);
            }
        }
        relations.sort_by(|left, right| {
            relation_key(left)
                .cmp(&relation_key(right))
                .then_with(|| left.phase_anchor.re.total_cmp(&right.phase_anchor.re))
                .then_with(|| left.phase_anchor.im.total_cmp(&right.phase_anchor.im))
        });
        if relations
            .windows(2)
            .any(|pair| relation_key(&pair[0]) == relation_key(&pair[1]))
        {
            return Err(SurfaceFragmentBundleError::DuplicateRelation);
        }
        for atom in &program_atoms {
            if [
                atom.output_local_role,
                atom.source_a_local_role,
                atom.source_b_local_role,
            ]
            .into_iter()
            .any(|role| usize::from(role) >= roles.len())
            {
                return Err(SurfaceFragmentBundleError::InvalidLocalRole);
            }
        }
        program_atoms.sort_unstable();
        program_atoms.dedup();
        Ok(Self {
            lineage_sha256,
            surface_sha256,
            roles: roles.into_boxed_slice(),
            relations: relations.into_boxed_slice(),
            program_atoms: program_atoms.into_boxed_slice(),
        })
    }

    #[must_use]
    pub const fn lineage_sha256(&self) -> &Commitment256 {
        &self.lineage_sha256
    }

    #[must_use]
    pub const fn surface_sha256(&self) -> &Commitment256 {
        &self.surface_sha256
    }

    #[must_use]
    pub fn roles(&self) -> &[StructuralRoleSignature] {
        &self.roles
    }

    #[must_use]
    pub fn relations(&self) -> &[LocalRelationFragment] {
        &self.relations
    }

    #[must_use]
    pub fn program_atoms(&self) -> &[TypedProgramAtom] {
        &self.program_atoms
    }
}

impl StructuralRoleCanonicalizer {
    #[must_use]
    pub fn colors(bundle: &SurfaceFragmentBundle, rounds: usize) -> Box<[Commitment256]> {
        let mut colors = bundle
            .roles
            .iter()
            .map(role_signature_commitment)
            .collect::<Vec<_>>();
        for _ in 0..rounds.min(OPERATOR_ROLE_COLOR_ROUNDS) {
            let previous = colors.clone();
            for (local_role, color) in colors.iter_mut().enumerate() {
                let mut neighborhood = bundle
                    .relations
                    .iter()
                    .filter_map(|relation| {
                        let (direction, neighbor) =
                            if usize::from(relation.source_local_role) == local_role {
                                (0_u8, relation.target_local_role)
                            } else if usize::from(relation.target_local_role) == local_role {
                                (1_u8, relation.source_local_role)
                            } else {
                                return None;
                            };
                        Some((
                            relation.plane,
                            direction,
                            relation.state as i8 as u8,
                            previous[usize::from(neighbor)],
                        ))
                    })
                    .collect::<Vec<_>>();
                neighborhood.sort_unstable();
                let mut hasher = Sha256::new();
                hasher.update(b"nando.role-color.v1");
                hasher.update(previous[local_role]);
                for (plane, direction, state, neighbor_color) in neighborhood {
                    hasher.update([plane, direction, state]);
                    hasher.update(neighbor_color);
                }
                *color = hasher.finalize().into();
            }
        }
        colors.into_boxed_slice()
    }
}

impl BoundedRoleAligner {
    #[must_use]
    pub fn align(
        bundles: &[SurfaceFragmentBundle],
        config: RoleAlignmentConfig,
    ) -> RoleAlignmentReport {
        if bundles.len() < 2 {
            return blocked_alignment(RoleAlignmentBlocker::TooFewBundles);
        }
        if bundles.len() > OPERATOR_BLUEPRINT_MAX_BUNDLES
            || config.max_hypotheses == 0
            || config.max_hypotheses > OPERATOR_BLUEPRINT_MAX_ALIGNMENTS
            || config.max_expansions == 0
            || config.max_expansions > OPERATOR_BLUEPRINT_MAX_EXPANSIONS
            || config.color_rounds == 0
            || config.color_rounds > OPERATOR_ROLE_COLOR_ROUNDS
        {
            return blocked_alignment(RoleAlignmentBlocker::InvalidConfig);
        }
        let unique_lineages = bundles
            .iter()
            .map(|bundle| bundle.lineage_sha256)
            .collect::<BTreeSet<_>>();
        if unique_lineages.len() != bundles.len() {
            return blocked_alignment(RoleAlignmentBlocker::DuplicateLineage);
        }

        let colors = bundles
            .iter()
            .map(|bundle| StructuralRoleCanonicalizer::colors(bundle, config.color_rounds))
            .collect::<Vec<_>>();
        let first = &bundles[0];
        let mut states = vec![AlignmentState {
            bindings: (0..first.roles.len())
                .map(|role| RoleBinding {
                    bundle_index: 0,
                    local_role: role as u8,
                    canonical_role: role as u8,
                })
                .collect(),
            canonical_role_count: first.roles.len() as u8,
        }];
        let mut expansions = 0_usize;
        let mut symmetric_branches = 0_usize;

        for bundle_index in 1..bundles.len() {
            let mut next = Vec::new();
            for state in states {
                let mut partial = vec![state];
                for local_role in 0..bundles[bundle_index].roles.len() {
                    let mut expanded = Vec::new();
                    for candidate in partial {
                        let compatible = compatible_canonical_roles(
                            &candidate,
                            bundles,
                            &colors,
                            bundle_index,
                            local_role,
                        );
                        if compatible.len() > 1 {
                            symmetric_branches = symmetric_branches
                                .saturating_add(compatible.len().saturating_sub(1));
                        }
                        for canonical_role in compatible {
                            expansions = expansions.saturating_add(1);
                            if expansions > config.max_expansions {
                                return RoleAlignmentReport {
                                    hypotheses: Box::new([]),
                                    expansions,
                                    symmetric_branches,
                                    complete: false,
                                    blocker: Some(RoleAlignmentBlocker::BudgetExhausted),
                                };
                            }
                            let mut branch = candidate.clone();
                            branch.bindings.push(RoleBinding {
                                bundle_index: bundle_index as u8,
                                local_role: local_role as u8,
                                canonical_role,
                            });
                            if canonical_role == branch.canonical_role_count {
                                if usize::from(branch.canonical_role_count)
                                    == OPERATOR_BLUEPRINT_MAX_ROLES
                                {
                                    continue;
                                }
                                branch.canonical_role_count =
                                    branch.canonical_role_count.saturating_add(1);
                            }
                            expanded.push(branch);
                            if expanded.len() >= config.max_hypotheses {
                                break;
                            }
                        }
                        if expanded.len() >= config.max_hypotheses {
                            break;
                        }
                    }
                    partial = deduplicate_alignment_states(expanded, config.max_hypotheses);
                    if partial.is_empty() {
                        return blocked_alignment(RoleAlignmentBlocker::NoCompatibleAlignment);
                    }
                }
                next.extend(partial);
                if next.len() >= config.max_hypotheses {
                    break;
                }
            }
            states = deduplicate_alignment_states(next, config.max_hypotheses);
        }

        let mut hypotheses = states
            .into_iter()
            .map(|mut state| {
                state.bindings.sort_unstable();
                let fingerprint_sha256 = alignment_commitment(&state.bindings);
                RoleAlignmentHypothesis {
                    bindings: state.bindings.into_boxed_slice(),
                    canonical_role_count: state.canonical_role_count,
                    fingerprint_sha256,
                }
            })
            .collect::<Vec<_>>();
        hypotheses.sort_by_key(|hypothesis| hypothesis.fingerprint_sha256);
        hypotheses.dedup_by_key(|hypothesis| hypothesis.fingerprint_sha256);
        RoleAlignmentReport {
            hypotheses: hypotheses.into_boxed_slice(),
            expansions,
            symmetric_branches,
            complete: true,
            blocker: None,
        }
    }
}

impl RoleAlignmentHypothesis {
    #[must_use]
    pub fn bindings(&self) -> &[RoleBinding] {
        &self.bindings
    }

    #[must_use]
    pub const fn canonical_role_count(&self) -> u8 {
        self.canonical_role_count
    }

    #[must_use]
    pub const fn fingerprint_sha256(&self) -> &Commitment256 {
        &self.fingerprint_sha256
    }

    #[must_use]
    pub fn canonical_role(&self, bundle_index: u8, local_role: u8) -> Option<u8> {
        self.bindings
            .binary_search_by_key(&(bundle_index, local_role), |binding| {
                (binding.bundle_index, binding.local_role)
            })
            .ok()
            .map(|index| self.bindings[index].canonical_role)
    }

    pub fn map_program_atom(
        &self,
        bundle_index: u8,
        atom: TypedProgramAtom,
    ) -> Option<TransformOp8> {
        Some(TransformOp8 {
            opcode: atom.opcode,
            output: self.canonical_role(bundle_index, atom.output_local_role)?,
            source_a: self.canonical_role(bundle_index, atom.source_a_local_role)?,
            source_b: self.canonical_role(bundle_index, atom.source_b_local_role)?,
            parameter: atom.parameter,
            flags: atom.flags,
        })
    }
}

impl BoundedCircuitBeam {
    #[must_use]
    pub fn synthesize(
        bundles: &[SurfaceFragmentBundle],
        alignments: &RoleAlignmentReport,
        config: BlueprintBeamConfig,
    ) -> BlueprintSynthesisReport {
        let mut blocker_counts = BTreeMap::new();
        if !alignments.complete {
            add_blueprint_blocker(
                &mut blocker_counts,
                BlueprintSynthesisBlocker::AlignmentIncomplete,
            );
            return blueprint_report(Vec::new(), 0, false, blocker_counts);
        }
        if config.max_blueprints == 0
            || config.max_blueprints > OPERATOR_BLUEPRINT_MAX_ALIGNMENTS
            || config.max_depth == 0
            || config.max_depth > OPERATOR_BLUEPRINT_MAX_BEAM_DEPTH
            || config.max_expansions == 0
            || config.max_expansions > OPERATOR_BLUEPRINT_MAX_EXPANSIONS
        {
            add_blueprint_blocker(
                &mut blocker_counts,
                BlueprintSynthesisBlocker::InvalidConfig,
            );
            return blueprint_report(Vec::new(), 0, false, blocker_counts);
        }

        let mut blueprints = BTreeMap::<Commitment256, CandidateOperatorBlueprint>::new();
        let mut expansions = 0_usize;
        let mut complete = true;

        for alignment in &alignments.hypotheses {
            let mapped = mapped_relation_modes(bundles, alignment);
            if mapped.is_empty() {
                add_blueprint_blocker(
                    &mut blocker_counts,
                    BlueprintSynthesisBlocker::EmptyRelations,
                );
                continue;
            }
            let Some(ordered_cells) = dependency_connected_cells(mapped.keys().copied()) else {
                add_blueprint_blocker(
                    &mut blocker_counts,
                    BlueprintSynthesisBlocker::DisconnectedRelations,
                );
                continue;
            };
            if ordered_cells.len() > config.max_depth {
                complete = false;
                add_blueprint_blocker(
                    &mut blocker_counts,
                    BlueprintSynthesisBlocker::BeamDepthReached,
                );
                continue;
            }

            let mut beam = vec![Vec::<OperatorCircuitRelation>::new()];
            for cell in ordered_cells {
                let mut expanded = BTreeMap::<Commitment256, Vec<OperatorCircuitRelation>>::new();
                for partial in beam {
                    for (mode, aggregate) in &mapped[&cell] {
                        expansions = expansions.saturating_add(1);
                        if expansions > config.max_expansions {
                            complete = false;
                            add_blueprint_blocker(
                                &mut blocker_counts,
                                BlueprintSynthesisBlocker::ExpansionBudgetReached,
                            );
                            break;
                        }
                        let magnitude = aggregate.re.hypot(aggregate.im);
                        if magnitude <= f64::EPSILON {
                            continue;
                        }
                        let mut branch = partial.clone();
                        branch.push(OperatorCircuitRelation {
                            cell,
                            state: mode.state,
                            phase_anchor: PhaseCenterCell {
                                re: aggregate.re / magnitude,
                                im: aggregate.im / magnitude,
                            },
                        });
                        expanded
                            .entry(relation_assignment_commitment(&branch))
                            .or_insert(branch);
                        if expanded.len() >= config.max_blueprints {
                            break;
                        }
                    }
                    if expansions > config.max_expansions || expanded.len() >= config.max_blueprints
                    {
                        break;
                    }
                }
                beam = expanded.into_values().collect();
                if !complete || beam.is_empty() {
                    break;
                }
            }
            if !complete {
                break;
            }

            let Some(transform_program) = mapped_transform_program(bundles, alignment) else {
                add_blueprint_blocker(
                    &mut blocker_counts,
                    BlueprintSynthesisBlocker::TransformCapacityReached,
                );
                continue;
            };
            let composition_dag = composition_dag(&transform_program);
            for mut relations in beam {
                canonicalize_phase_gauge(&mut relations);
                match OperatorCircuit::new(alignment.canonical_role_count, relations) {
                    Ok(relation_program) => {
                        let role_graph = RoleGraph {
                            role_count: alignment.canonical_role_count,
                            bindings: alignment.bindings.clone(),
                            canonical_roles: canonical_role_signatures(bundles, alignment),
                        };
                        let renderer_hypothesis = RendererContract {
                            commitment_sha256: renderer_commitment(&transform_program),
                        };
                        let verifier_contract = VerifierContract {
                            commitment_sha256: verifier_commitment(
                                &relation_program,
                                &transform_program,
                            ),
                        };
                        let fingerprint_sha256 = blueprint_commitment(
                            &role_graph,
                            &relation_program,
                            &transform_program,
                            &composition_dag,
                            &renderer_hypothesis,
                            &verifier_contract,
                        );
                        blueprints.entry(fingerprint_sha256).or_insert(
                            CandidateOperatorBlueprint {
                                role_graph,
                                relation_program,
                                transform_program: transform_program.clone().into_boxed_slice(),
                                composition_dag: composition_dag.clone(),
                                renderer_hypothesis,
                                verifier_contract,
                                fingerprint_sha256,
                            },
                        );
                        if blueprints.len() >= config.max_blueprints {
                            break;
                        }
                    }
                    Err(OperatorCircuitError::DisconnectedCircuit) => add_blueprint_blocker(
                        &mut blocker_counts,
                        BlueprintSynthesisBlocker::DisconnectedRelations,
                    ),
                    Err(_) => add_blueprint_blocker(
                        &mut blocker_counts,
                        BlueprintSynthesisBlocker::InvalidCircuit,
                    ),
                }
            }
            if blueprints.len() >= config.max_blueprints {
                break;
            }
        }

        if blueprints.is_empty() {
            add_blueprint_blocker(&mut blocker_counts, BlueprintSynthesisBlocker::NoBlueprint);
        }
        blueprint_report(
            blueprints.into_values().collect(),
            expansions,
            complete,
            blocker_counts,
        )
    }
}

impl CandidateOperatorBlueprint {
    #[must_use]
    pub const fn role_graph(&self) -> &RoleGraph {
        &self.role_graph
    }

    #[must_use]
    pub const fn relation_program(&self) -> &OperatorCircuit {
        &self.relation_program
    }

    #[must_use]
    pub fn transform_program(&self) -> &[TransformOp8] {
        &self.transform_program
    }

    #[must_use]
    pub const fn composition_dag(&self) -> &CompositionDag {
        &self.composition_dag
    }

    #[must_use]
    pub const fn renderer_hypothesis(&self) -> &RendererContract {
        &self.renderer_hypothesis
    }

    #[must_use]
    pub const fn verifier_contract(&self) -> &VerifierContract {
        &self.verifier_contract
    }

    #[must_use]
    pub const fn fingerprint_sha256(&self) -> &Commitment256 {
        &self.fingerprint_sha256
    }
}

impl RoleGraph {
    #[must_use]
    pub const fn role_count(&self) -> u8 {
        self.role_count
    }

    #[must_use]
    pub fn bindings(&self) -> &[RoleBinding] {
        &self.bindings
    }

    #[must_use]
    pub fn canonical_roles(&self) -> &[StructuralRoleSignature] {
        &self.canonical_roles
    }
}

impl CompositionDag {
    #[must_use]
    pub fn edges(&self) -> &[CompositionEdge] {
        &self.edges
    }
}

impl RendererContract {
    #[must_use]
    pub const fn commitment_sha256(&self) -> &Commitment256 {
        &self.commitment_sha256
    }
}

impl VerifierContract {
    #[must_use]
    pub const fn commitment_sha256(&self) -> &Commitment256 {
        &self.commitment_sha256
    }
}

impl FrozenOperatorBlueprintSet {
    pub fn freeze(
        source_generation: u64,
        bundles: &[SurfaceFragmentBundle],
        config: BlueprintBeamConfig,
        report: &BlueprintSynthesisReport,
    ) -> Result<Self, FrozenBlueprintError> {
        if !report.complete {
            return Err(FrozenBlueprintError::IncompleteSynthesis);
        }
        if report.blueprints.is_empty() {
            return Err(FrozenBlueprintError::EmptyCandidateSet);
        }
        let support_lineages_sha256 = bundles
            .iter()
            .map(|bundle| bundle.lineage_sha256)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let candidate_set_sha256 = candidate_set_commitment(&report.blueprints);
        Ok(Self {
            source_generation,
            canonicalizer_version: OPERATOR_BLUEPRINT_CANONICALIZER_VERSION,
            config,
            support_lineages_sha256,
            candidate_set_sha256,
            blueprints: report.blueprints.clone(),
        })
    }

    #[must_use]
    pub fn future_window(&self) -> FrozenBlueprintFutureWindow {
        FrozenBlueprintFutureWindow {
            frozen: self.clone(),
            future_lineages_sha256: BTreeSet::new(),
        }
    }

    #[must_use]
    pub const fn source_generation(&self) -> u64 {
        self.source_generation
    }

    #[must_use]
    pub const fn canonicalizer_version(&self) -> u32 {
        self.canonicalizer_version
    }

    #[must_use]
    pub const fn config(&self) -> BlueprintBeamConfig {
        self.config
    }

    #[must_use]
    pub fn support_lineages_sha256(&self) -> &[Commitment256] {
        &self.support_lineages_sha256
    }

    #[must_use]
    pub const fn candidate_set_sha256(&self) -> &Commitment256 {
        &self.candidate_set_sha256
    }

    #[must_use]
    pub fn blueprints(&self) -> &[CandidateOperatorBlueprint] {
        &self.blueprints
    }
}

impl FrozenBlueprintFutureWindow {
    pub fn admit_lineage(
        &mut self,
        bundle: &SurfaceFragmentBundle,
    ) -> Result<(), FrozenBlueprintError> {
        if self
            .frozen
            .support_lineages_sha256
            .binary_search(&bundle.lineage_sha256)
            .is_ok()
        {
            return Err(FrozenBlueprintError::SupportLineageReused);
        }
        if !self.future_lineages_sha256.insert(bundle.lineage_sha256) {
            return Err(FrozenBlueprintError::DuplicateFutureLineage);
        }
        Ok(())
    }

    #[must_use]
    pub const fn frozen(&self) -> &FrozenOperatorBlueprintSet {
        &self.frozen
    }

    #[must_use]
    pub fn future_lineages_sha256(&self) -> &BTreeSet<Commitment256> {
        &self.future_lineages_sha256
    }
}

impl BlueprintFutureEvaluator {
    #[must_use]
    pub fn evaluate(
        frozen: &FrozenOperatorBlueprintSet,
        future_bundles: &[SurfaceFragmentBundle],
        config: BlueprintFutureConfig,
        control: BlueprintPhaseControl,
    ) -> BlueprintFutureReport {
        if config.min_lineages_per_edge == 0
            || !(0.0..=1.0).contains(&config.coherence_floor)
            || !(0.0..=1.0).contains(&config.coherence_margin)
        {
            return blocked_future(control, BlueprintFutureBlocker::InvalidConfig);
        }
        let mut window = frozen.future_window();
        for bundle in future_bundles {
            match window.admit_lineage(bundle) {
                Ok(()) => {}
                Err(FrozenBlueprintError::SupportLineageReused) => {
                    return blocked_future(control, BlueprintFutureBlocker::SupportLineageReused);
                }
                Err(FrozenBlueprintError::DuplicateFutureLineage) => {
                    return blocked_future(control, BlueprintFutureBlocker::DuplicateFutureLineage);
                }
                Err(_) => return blocked_future(control, BlueprintFutureBlocker::InvalidConfig),
            }
        }

        let controlled_phases = controlled_future_phases(future_bundles, control);
        let mut scores = frozen
            .blueprints
            .iter()
            .map(|blueprint| {
                score_blueprint_future(
                    blueprint,
                    future_bundles,
                    &controlled_phases,
                    config,
                    control,
                )
            })
            .collect::<Vec<_>>();
        scores.sort_by(|left, right| {
            right
                .whole_circuit_coherence
                .total_cmp(&left.whole_circuit_coherence)
                .then_with(|| {
                    left.blueprint_fingerprint_sha256
                        .cmp(&right.blueprint_fingerprint_sha256)
                })
        });
        let Some(best) = scores.first() else {
            return blocked_future(control, BlueprintFutureBlocker::NoEligibleBlueprint);
        };
        if !best.eligible {
            return BlueprintFutureReport {
                control,
                scores: scores.into_boxed_slice(),
                winner_fingerprint_sha256: None,
                runner_up_margin: 0.0,
                blocker: Some(BlueprintFutureBlocker::NoEligibleBlueprint),
            };
        }
        if best.whole_circuit_coherence < config.coherence_floor {
            return BlueprintFutureReport {
                control,
                scores: scores.into_boxed_slice(),
                winner_fingerprint_sha256: None,
                runner_up_margin: 0.0,
                blocker: Some(BlueprintFutureBlocker::CoherenceBelowFloor),
            };
        }
        let runner_up = scores
            .iter()
            .skip(1)
            .find(|score| score.eligible)
            .map_or(0.0, |score| score.whole_circuit_coherence);
        let margin = best.whole_circuit_coherence - runner_up;
        if margin < config.coherence_margin {
            return BlueprintFutureReport {
                control,
                scores: scores.into_boxed_slice(),
                winner_fingerprint_sha256: None,
                runner_up_margin: margin,
                blocker: Some(BlueprintFutureBlocker::InsufficientRunnerUpMargin),
            };
        }
        let winner = best.blueprint_fingerprint_sha256;
        BlueprintFutureReport {
            control,
            scores: scores.into_boxed_slice(),
            winner_fingerprint_sha256: Some(winner),
            runner_up_margin: margin,
            blocker: None,
        }
    }
}

fn score_blueprint_future(
    blueprint: &CandidateOperatorBlueprint,
    future_bundles: &[SurfaceFragmentBundle],
    controlled_phases: &BTreeMap<(usize, usize), Option<PhaseCenterCell>>,
    config: BlueprintFutureConfig,
    control: BlueprintPhaseControl,
) -> BlueprintFutureScore {
    let relations = blueprint.relation_program.relations();
    let mut edge_samples = vec![BTreeMap::<Commitment256, PhaseCenterCell>::new(); relations.len()];
    let mut ambiguous_bindings = 0_usize;

    for (bundle_index, bundle) in future_bundles.iter().enumerate() {
        let mappings = future_role_mappings(bundle, &blueprint.role_graph, 2);
        if mappings.len() != 1 {
            ambiguous_bindings = ambiguous_bindings.saturating_add(1);
            continue;
        }
        let mapping = &mappings[0];
        for (relation_index, observed) in bundle.relations.iter().enumerate() {
            let Some(sample_phase) = controlled_phases
                .get(&(bundle_index, relation_index))
                .copied()
                .flatten()
            else {
                continue;
            };
            let cell = OperatorRelationCell {
                plane: observed.plane,
                source_role: mapping[usize::from(observed.source_local_role)],
                target_role: mapping[usize::from(observed.target_local_role)],
            };
            let Some((edge_index, expected)) = relations
                .iter()
                .enumerate()
                .find(|(_, expected)| expected.cell == cell && expected.state == observed.state)
            else {
                continue;
            };
            let expected_anchor = match control {
                BlueprintPhaseControl::MatchedRandomCenter => random_phase_anchor(expected.cell),
                _ => expected.phase_anchor,
            };
            let aligned = align_phase(sample_phase, expected_anchor);
            edge_samples[edge_index]
                .entry(bundle.lineage_sha256)
                .or_insert(aligned);
        }
    }

    let mut edge_coherences = Vec::with_capacity(relations.len());
    let mut plane_edges = BTreeMap::<u8, Vec<(f64, PhaseCenterCell)>>::new();
    let mut covered_edges = 0_usize;
    for (edge_index, samples) in edge_samples.iter().enumerate() {
        let (coherence, direction) = if samples.len() >= config.min_lineages_per_edge {
            covered_edges = covered_edges.saturating_add(1);
            resultant_summary(samples.values().copied())
        } else {
            (0.0, PhaseCenterCell::default())
        };
        edge_coherences.push(coherence);
        plane_edges
            .entry(relations[edge_index].cell.plane)
            .or_default()
            .push((coherence, direction));
    }
    let plane_summaries = plane_edges
        .values()
        .map(|edges| {
            let edge_strengths = edges
                .iter()
                .map(|(coherence, _)| *coherence)
                .collect::<Vec<_>>();
            let (closure, direction) =
                resultant_summary(edges.iter().map(|(_, direction)| *direction));
            (geometric_mean(&edge_strengths) * closure, direction)
        })
        .collect::<Vec<_>>();
    let plane_coherences = plane_summaries
        .iter()
        .map(|(coherence, _)| *coherence)
        .collect::<Vec<_>>();
    let covered_planes = plane_coherences
        .iter()
        .filter(|coherence| **coherence > 0.0)
        .count();
    let edge_mean = geometric_mean(&edge_coherences);
    let plane_mean = geometric_mean(&plane_coherences);
    let (cross_plane_closure, _) =
        resultant_summary(plane_summaries.iter().map(|(_, direction)| *direction));
    let whole_circuit_coherence = edge_mean * plane_mean * cross_plane_closure;
    let required_planes = relations
        .iter()
        .map(|relation| relation.cell.plane)
        .collect::<BTreeSet<_>>()
        .len();
    let eligible = ambiguous_bindings == 0
        && covered_edges == relations.len()
        && covered_planes == required_planes
        && whole_circuit_coherence.is_finite();

    BlueprintFutureScore {
        blueprint_fingerprint_sha256: blueprint.fingerprint_sha256,
        edge_coherences: edge_coherences.into_boxed_slice(),
        plane_coherences: plane_coherences.into_boxed_slice(),
        whole_circuit_coherence,
        covered_edges,
        covered_planes,
        ambiguous_bindings,
        eligible,
    }
}

fn canonical_role_signatures(
    bundles: &[SurfaceFragmentBundle],
    alignment: &RoleAlignmentHypothesis,
) -> Box<[StructuralRoleSignature]> {
    let mut roles = Vec::with_capacity(usize::from(alignment.canonical_role_count));
    for canonical_role in 0..alignment.canonical_role_count {
        let binding = alignment
            .bindings
            .iter()
            .find(|binding| binding.canonical_role == canonical_role)
            .expect("alignment owns every canonical role");
        roles.push(
            bundles[usize::from(binding.bundle_index)].roles[usize::from(binding.local_role)]
                .clone(),
        );
    }
    roles.into_boxed_slice()
}

fn future_role_mappings(
    bundle: &SurfaceFragmentBundle,
    role_graph: &RoleGraph,
    limit: usize,
) -> Vec<Vec<u8>> {
    fn visit(
        bundle: &SurfaceFragmentBundle,
        role_graph: &RoleGraph,
        local_role: usize,
        current: &mut Vec<u8>,
        used: &mut BTreeSet<u8>,
        limit: usize,
        output: &mut Vec<Vec<u8>>,
    ) {
        if output.len() >= limit {
            return;
        }
        if local_role == bundle.roles.len() {
            output.push(current.clone());
            return;
        }
        for (canonical_role, signature) in role_graph.canonical_roles.iter().enumerate() {
            let canonical_role = canonical_role as u8;
            if used.contains(&canonical_role)
                || !role_signatures_compatible(&bundle.roles[local_role], signature)
            {
                continue;
            }
            used.insert(canonical_role);
            current.push(canonical_role);
            visit(
                bundle,
                role_graph,
                local_role + 1,
                current,
                used,
                limit,
                output,
            );
            current.pop();
            used.remove(&canonical_role);
            if output.len() >= limit {
                return;
            }
        }
    }

    let mut output = Vec::new();
    visit(
        bundle,
        role_graph,
        0,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        limit,
        &mut output,
    );
    output
}

fn controlled_future_phases(
    bundles: &[SurfaceFragmentBundle],
    control: BlueprintPhaseControl,
) -> BTreeMap<(usize, usize), Option<PhaseCenterCell>> {
    let mut indexed = bundles
        .iter()
        .enumerate()
        .flat_map(|(bundle_index, bundle)| {
            bundle
                .relations
                .iter()
                .enumerate()
                .map(move |(relation_index, relation)| {
                    ((bundle_index, relation_index), relation.phase_anchor)
                })
        })
        .collect::<Vec<_>>();
    let original = indexed.iter().map(|(_, phase)| *phase).collect::<Vec<_>>();
    for (index, (_, phase)) in indexed.iter_mut().enumerate() {
        *phase = match control {
            BlueprintPhaseControl::Full | BlueprintPhaseControl::MatchedRandomCenter => {
                original[index]
            }
            BlueprintPhaseControl::NoPhase => PhaseCenterCell::default(),
            BlueprintPhaseControl::ShuffledPhase => original[(index + 1) % original.len()],
            BlueprintPhaseControl::MagnitudeOnly => PhaseCenterCell { re: 1.0, im: 0.0 },
        };
    }
    indexed
        .into_iter()
        .map(|(key, phase)| {
            let value = (phase.re.hypot(phase.im) > f64::EPSILON).then_some(phase);
            (key, value)
        })
        .collect()
}

fn align_phase(observed: PhaseCenterCell, expected: PhaseCenterCell) -> PhaseCenterCell {
    PhaseCenterCell {
        re: observed.re * expected.re + observed.im * expected.im,
        im: observed.im * expected.re - observed.re * expected.im,
    }
}

fn random_phase_anchor(cell: OperatorRelationCell) -> PhaseCenterCell {
    let mixed = u32::from(cell.plane)
        .wrapping_mul(0x9e37_79b9)
        .wrapping_add(u32::from(cell.source_role).wrapping_mul(0x85eb_ca6b))
        .wrapping_add(u32::from(cell.target_role).wrapping_mul(0xc2b2_ae35));
    let angle = (f64::from(mixed) / f64::from(u32::MAX)) * std::f64::consts::TAU;
    PhaseCenterCell {
        re: angle.cos(),
        im: angle.sin(),
    }
}

fn resultant_summary(phases: impl Iterator<Item = PhaseCenterCell>) -> (f64, PhaseCenterCell) {
    let mut re = 0.0;
    let mut im = 0.0;
    let mut count = 0_usize;
    for phase in phases {
        let magnitude = phase.re.hypot(phase.im);
        if magnitude <= f64::EPSILON {
            continue;
        }
        re += phase.re / magnitude;
        im += phase.im / magnitude;
        count = count.saturating_add(1);
    }
    if count == 0 {
        (0.0, PhaseCenterCell::default())
    } else {
        let magnitude = re.hypot(im);
        if magnitude <= f64::EPSILON {
            (0.0, PhaseCenterCell::default())
        } else {
            (
                magnitude / count as f64,
                PhaseCenterCell {
                    re: re / magnitude,
                    im: im / magnitude,
                },
            )
        }
    }
}

fn geometric_mean(values: &[f64]) -> f64 {
    if values.is_empty()
        || values
            .iter()
            .any(|value| *value <= 0.0 || !value.is_finite())
    {
        return 0.0;
    }
    (values.iter().map(|value| value.ln()).sum::<f64>() / values.len() as f64).exp()
}

fn blocked_future(
    control: BlueprintPhaseControl,
    blocker: BlueprintFutureBlocker,
) -> BlueprintFutureReport {
    BlueprintFutureReport {
        control,
        scores: Box::new([]),
        winner_fingerprint_sha256: None,
        runner_up_margin: 0.0,
        blocker: Some(blocker),
    }
}

fn mapped_relation_modes(
    bundles: &[SurfaceFragmentBundle],
    alignment: &RoleAlignmentHypothesis,
) -> BTreeMap<OperatorRelationCell, BTreeMap<PhaseModeKey, PhaseModeAggregate>> {
    let mut mapped = BTreeMap::new();
    for (bundle_index, bundle) in bundles.iter().enumerate() {
        for relation in &bundle.relations {
            let Some(source_role) =
                alignment.canonical_role(bundle_index as u8, relation.source_local_role)
            else {
                continue;
            };
            let Some(target_role) =
                alignment.canonical_role(bundle_index as u8, relation.target_local_role)
            else {
                continue;
            };
            let cell = OperatorRelationCell {
                plane: relation.plane,
                source_role,
                target_role,
            };
            let mode = PhaseModeKey {
                state: relation.state,
                slot: phase_slot(relation.phase_anchor),
            };
            let aggregate = mapped
                .entry(cell)
                .or_insert_with(BTreeMap::new)
                .entry(mode)
                .or_insert_with(PhaseModeAggregate::default);
            aggregate.re += relation.phase_anchor.re;
            aggregate.im += relation.phase_anchor.im;
        }
    }
    mapped
}

fn dependency_connected_cells(
    cells: impl Iterator<Item = OperatorRelationCell>,
) -> Option<Vec<OperatorRelationCell>> {
    let mut remaining = cells.collect::<BTreeSet<_>>();
    let first = remaining.pop_first()?;
    let mut ordered = vec![first];
    let mut active_roles = BTreeSet::from([first.source_role, first.target_role]);
    while !remaining.is_empty() {
        let next = remaining
            .iter()
            .find(|cell| {
                active_roles.contains(&cell.source_role) || active_roles.contains(&cell.target_role)
            })
            .copied()?;
        remaining.remove(&next);
        active_roles.insert(next.source_role);
        active_roles.insert(next.target_role);
        ordered.push(next);
    }
    Some(ordered)
}

fn mapped_transform_program(
    bundles: &[SurfaceFragmentBundle],
    alignment: &RoleAlignmentHypothesis,
) -> Option<Vec<TransformOp8>> {
    let mut encoded = BTreeMap::<[u8; 8], TransformOp8>::new();
    for (bundle_index, bundle) in bundles.iter().enumerate() {
        for atom in &bundle.program_atoms {
            let transform = alignment.map_program_atom(bundle_index as u8, *atom)?;
            encoded.entry(transform.encode()).or_insert(transform);
            if encoded.len() > OPERATOR_BLUEPRINT_MAX_PROGRAM_ATOMS {
                return None;
            }
        }
    }
    Some(encoded.into_values().collect())
}

fn composition_dag(transforms: &[TransformOp8]) -> CompositionDag {
    let mut edges = BTreeSet::new();
    for (producer_index, producer) in transforms.iter().enumerate() {
        for (consumer_index, consumer) in transforms.iter().enumerate() {
            if producer_index == consumer_index {
                continue;
            }
            if producer.output == consumer.source_a || producer.output == consumer.source_b {
                edges.insert(CompositionEdge {
                    producer_step: producer_index as u8,
                    consumer_step: consumer_index as u8,
                });
            }
        }
    }
    CompositionDag {
        edges: edges.into_iter().collect::<Vec<_>>().into_boxed_slice(),
    }
}

fn phase_slot(phase: PhaseCenterCell) -> u8 {
    const SLOTS: u8 = 32;
    let turns = phase.im.atan2(phase.re) / std::f64::consts::TAU;
    let normalized = if turns < 0.0 { turns + 1.0 } else { turns };
    ((normalized * f64::from(SLOTS)).round() as u8) % SLOTS
}

fn relation_assignment_commitment(relations: &[OperatorCircuitRelation]) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.relation-assignment.v1");
    for relation in relations {
        update_relation_hasher(&mut hasher, relation);
    }
    hasher.finalize().into()
}

fn canonicalize_phase_gauge(relations: &mut [OperatorCircuitRelation]) {
    relations.sort_by_key(|relation| relation.cell);
    let Some(origin) = relations.first().map(|relation| relation.phase_anchor) else {
        return;
    };
    for relation in relations {
        relation.phase_anchor = align_phase(relation.phase_anchor, origin);
    }
}

fn renderer_commitment(transforms: &[TransformOp8]) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.renderer-contract.v1");
    for transform in transforms {
        hasher.update(transform.encode());
    }
    hasher.finalize().into()
}

fn verifier_commitment(circuit: &OperatorCircuit, transforms: &[TransformOp8]) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.verifier-contract.v1");
    hasher.update([circuit.role_count()]);
    for relation in circuit.relations() {
        update_relation_hasher(&mut hasher, relation);
    }
    for transform in transforms {
        hasher.update(transform.encode());
    }
    hasher.finalize().into()
}

fn blueprint_commitment(
    role_graph: &RoleGraph,
    circuit: &OperatorCircuit,
    transforms: &[TransformOp8],
    composition: &CompositionDag,
    renderer: &RendererContract,
    verifier: &VerifierContract,
) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.operator-blueprint.v1");
    hasher.update([role_graph.role_count]);
    // Surface bindings are cold proof provenance. They must not make the
    // semantic identity of one transferable law depend on support history.
    for role in &role_graph.canonical_roles {
        hasher.update(role_signature_commitment(role));
    }
    for relation in circuit.relations() {
        update_relation_hasher(&mut hasher, relation);
    }
    for transform in transforms {
        hasher.update(transform.encode());
    }
    for edge in &composition.edges {
        hasher.update([edge.producer_step, edge.consumer_step]);
    }
    hasher.update(renderer.commitment_sha256);
    hasher.update(verifier.commitment_sha256);
    hasher.finalize().into()
}

fn candidate_set_commitment(blueprints: &[CandidateOperatorBlueprint]) -> Commitment256 {
    let mut fingerprints = blueprints
        .iter()
        .map(|blueprint| blueprint.fingerprint_sha256)
        .collect::<Vec<_>>();
    fingerprints.sort_unstable();
    let mut hasher = Sha256::new();
    hasher.update(b"nando.frozen-blueprint-set.v1");
    for fingerprint in fingerprints {
        hasher.update(fingerprint);
    }
    hasher.finalize().into()
}

fn update_relation_hasher(hasher: &mut Sha256, relation: &OperatorCircuitRelation) {
    hasher.update([
        relation.cell.plane,
        relation.cell.source_role,
        relation.cell.target_role,
        relation.state as i8 as u8,
    ]);
    hasher.update(relation.phase_anchor.re.to_bits().to_le_bytes());
    hasher.update(relation.phase_anchor.im.to_bits().to_le_bytes());
}

fn blueprint_report(
    mut blueprints: Vec<CandidateOperatorBlueprint>,
    expansions: usize,
    complete: bool,
    blocker_counts: BTreeMap<BlueprintSynthesisBlocker, usize>,
) -> BlueprintSynthesisReport {
    blueprints.sort_by_key(|blueprint| blueprint.fingerprint_sha256);
    BlueprintSynthesisReport {
        blueprints: blueprints.into_boxed_slice(),
        expansions,
        complete,
        blockers: blocker_counts
            .into_iter()
            .map(|(blocker, count)| BlueprintSynthesisBlockerCount { blocker, count })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    }
}

fn add_blueprint_blocker(
    blockers: &mut BTreeMap<BlueprintSynthesisBlocker, usize>,
    blocker: BlueprintSynthesisBlocker,
) {
    let count = blockers.entry(blocker).or_default();
    *count = count.saturating_add(1);
}

fn compatible_canonical_roles(
    state: &AlignmentState,
    bundles: &[SurfaceFragmentBundle],
    colors: &[Box<[Commitment256]>],
    bundle_index: usize,
    local_role: usize,
) -> Vec<u8> {
    let used_here = state
        .bindings
        .iter()
        .filter(|binding| usize::from(binding.bundle_index) == bundle_index)
        .map(|binding| binding.canonical_role)
        .collect::<BTreeSet<_>>();
    let signature = &bundles[bundle_index].roles[local_role];
    let color = colors[bundle_index][local_role];
    let mut exact = Vec::new();
    let mut structural = Vec::new();
    for canonical_role in 0..state.canonical_role_count {
        if used_here.contains(&canonical_role) {
            continue;
        }
        let Some(prototype) = state
            .bindings
            .iter()
            .find(|binding| binding.canonical_role == canonical_role)
        else {
            continue;
        };
        let prototype_signature =
            &bundles[usize::from(prototype.bundle_index)].roles[usize::from(prototype.local_role)];
        if !role_signatures_compatible(signature, prototype_signature) {
            continue;
        }
        let prototype_color =
            colors[usize::from(prototype.bundle_index)][usize::from(prototype.local_role)];
        if color == prototype_color {
            exact.push(canonical_role);
        } else {
            structural.push(canonical_role);
        }
    }
    exact.extend(structural);
    if exact.is_empty() && usize::from(state.canonical_role_count) < OPERATOR_BLUEPRINT_MAX_ROLES {
        exact.push(state.canonical_role_count);
    }
    exact
}

fn role_signatures_compatible(
    left: &StructuralRoleSignature,
    right: &StructuralRoleSignature,
) -> bool {
    left.type_class == right.type_class
        && left.cardinality_class == right.cardinality_class
        && left.temporal_position == right.temporal_position
        && left.constraint_mask == right.constraint_mask
        && relation_planes_overlap_or_empty(
            &left.neighboring_relation_planes,
            &right.neighboring_relation_planes,
        )
}

fn relation_planes_overlap_or_empty(left: &[u8], right: &[u8]) -> bool {
    left.is_empty()
        || right.is_empty()
        || left.iter().any(|plane| right.binary_search(plane).is_ok())
}

fn deduplicate_alignment_states(states: Vec<AlignmentState>, limit: usize) -> Vec<AlignmentState> {
    let mut by_fingerprint = BTreeMap::new();
    for mut state in states {
        state.bindings.sort_unstable();
        by_fingerprint
            .entry(alignment_commitment(&state.bindings))
            .or_insert(state);
        if by_fingerprint.len() >= limit {
            break;
        }
    }
    by_fingerprint.into_values().collect()
}

fn blocked_alignment(blocker: RoleAlignmentBlocker) -> RoleAlignmentReport {
    RoleAlignmentReport {
        hypotheses: Box::new([]),
        expansions: 0,
        symmetric_branches: 0,
        complete: false,
        blocker: Some(blocker),
    }
}

fn relation_key(relation: &LocalRelationFragment) -> (u8, u8, u8, i8) {
    (
        relation.plane,
        relation.source_local_role,
        relation.target_local_role,
        relation.state as i8,
    )
}

fn role_signature_commitment(role: &StructuralRoleSignature) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.structural-role.v1");
    hasher.update([
        role.type_class,
        role.cardinality_class,
        role.temporal_position,
    ]);
    hasher.update(role.constraint_mask.to_le_bytes());
    hasher.update((role.neighboring_relation_planes.len() as u16).to_le_bytes());
    hasher.update(&role.neighboring_relation_planes);
    hasher.finalize().into()
}

fn alignment_commitment(bindings: &[RoleBinding]) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.role-alignment.v1");
    for binding in bindings {
        hasher.update([
            binding.bundle_index,
            binding.local_role,
            binding.canonical_role,
        ]);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Commitment256 {
        [byte; 32]
    }

    fn role() -> StructuralRoleSignature {
        StructuralRoleSignature::new(1, 1, 0, 0, vec![0])
    }

    fn symmetric_bundle(lineage: u8, surface: u8, swap: bool) -> SurfaceFragmentBundle {
        let edge_a = if swap { (1, 2) } else { (0, 2) };
        let edge_b = if swap { (0, 2) } else { (1, 2) };
        SurfaceFragmentBundle::new(
            digest(lineage),
            digest(surface),
            vec![
                role(),
                role(),
                StructuralRoleSignature::new(2, 1, 0, 0, vec![0]),
            ],
            vec![
                LocalRelationFragment {
                    plane: 0,
                    source_local_role: edge_a.0,
                    target_local_role: edge_a.1,
                    state: TernaryRelationState::Supported,
                    phase_anchor: PhaseCenterCell { re: 1.0, im: 0.0 },
                },
                LocalRelationFragment {
                    plane: 0,
                    source_local_role: edge_b.0,
                    target_local_role: edge_b.1,
                    state: TernaryRelationState::Supported,
                    phase_anchor: PhaseCenterCell { re: 0.0, im: 1.0 },
                },
            ],
            Vec::new(),
        )
        .expect("valid local bundle")
    }

    #[test]
    fn local_role_symmetry_preserves_competing_alignments() {
        let report = BoundedRoleAligner::align(
            &[
                symmetric_bundle(1, 11, false),
                symmetric_bundle(2, 12, true),
            ],
            RoleAlignmentConfig::default(),
        );

        assert!(report.complete);
        assert_eq!(report.blocker, None);
        assert!(report.hypotheses.len() >= 2);
        assert!(report.symmetric_branches >= 1);
        assert!(
            report
                .hypotheses
                .iter()
                .all(|hypothesis| hypothesis.canonical_role_count() == 3)
        );
    }

    #[test]
    fn duplicate_full_lineage_is_rejected_before_alignment() {
        let report = BoundedRoleAligner::align(
            &[
                symmetric_bundle(1, 11, false),
                symmetric_bundle(1, 12, true),
            ],
            RoleAlignmentConfig::default(),
        );
        assert_eq!(report.blocker, Some(RoleAlignmentBlocker::DuplicateLineage));
        assert!(!report.complete);
    }

    #[test]
    fn caller_cannot_raise_alignment_budgets() {
        let report = BoundedRoleAligner::align(
            &[
                symmetric_bundle(1, 11, false),
                symmetric_bundle(2, 12, true),
            ],
            RoleAlignmentConfig {
                max_hypotheses: OPERATOR_BLUEPRINT_MAX_ALIGNMENTS + 1,
                ..RoleAlignmentConfig::default()
            },
        );
        assert_eq!(report.blocker, Some(RoleAlignmentBlocker::InvalidConfig));
    }

    #[test]
    fn symmetric_local_graphs_create_competing_frozen_blueprints() {
        let bundles = [
            symmetric_bundle(1, 11, false),
            symmetric_bundle(2, 12, true),
            symmetric_bundle(3, 13, false),
        ];
        let alignments = BoundedRoleAligner::align(&bundles, RoleAlignmentConfig::default());
        let synthesis =
            BoundedCircuitBeam::synthesize(&bundles, &alignments, BlueprintBeamConfig::default());

        assert!(synthesis.complete);
        assert!(synthesis.blueprints.len() >= 2);
        assert!(synthesis.blueprints.len() <= OPERATOR_BLUEPRINT_MAX_ALIGNMENTS);
        assert!(
            synthesis.blueprints.iter().all(|blueprint| blueprint
                .relation_program()
                .relations()
                .len()
                == 2)
        );

        let frozen = FrozenOperatorBlueprintSet::freeze(
            7,
            &bundles,
            BlueprintBeamConfig::default(),
            &synthesis,
        )
        .expect("frozen competing set");
        assert_eq!(frozen.support_lineages_sha256().len(), 3);
        assert_ne!(frozen.candidate_set_sha256(), &[0; 32]);

        let mut future = frozen.future_window();
        let copied_support = symmetric_bundle(1, 99, true);
        assert_eq!(
            future.admit_lineage(&copied_support),
            Err(FrozenBlueprintError::SupportLineageReused)
        );
        let independent = symmetric_bundle(4, 14, true);
        assert_eq!(future.admit_lineage(&independent), Ok(()));
        assert_eq!(
            future.admit_lineage(&independent),
            Err(FrozenBlueprintError::DuplicateFutureLineage)
        );
    }

    #[test]
    fn caller_cannot_raise_beam_budgets() {
        let bundles = [
            symmetric_bundle(1, 11, false),
            symmetric_bundle(2, 12, true),
        ];
        let alignments = BoundedRoleAligner::align(&bundles, RoleAlignmentConfig::default());
        let report = BoundedCircuitBeam::synthesize(
            &bundles,
            &alignments,
            BlueprintBeamConfig {
                max_expansions: OPERATOR_BLUEPRINT_MAX_EXPANSIONS + 1,
                ..BlueprintBeamConfig::default()
            },
        );
        assert!(!report.complete);
        assert_eq!(
            report.blockers.as_ref(),
            &[BlueprintSynthesisBlockerCount {
                blocker: BlueprintSynthesisBlocker::InvalidConfig,
                count: 1,
            }]
        );
    }
}
