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
pub const OPERATOR_ROLE_NONE: u8 = u8::MAX;
pub const OPERATOR_BLUEPRINT_CANONICALIZER_VERSION: u32 = 2;
pub const OPERATOR_BLUEPRINT_SCORE_SCALE: i64 = 1_000_000_000;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchStage {
    InputValidation,
    RoleAlignment,
    CircuitBeam,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchCompletion {
    Complete {
        explored: usize,
    },
    Exhausted {
        stage: SearchStage,
        explored: usize,
        frontier_remaining: usize,
    },
}

impl SearchCompletion {
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Complete { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleAlignmentReport {
    pub hypotheses: Box<[RoleAlignmentHypothesis]>,
    pub expansions: usize,
    pub symmetric_branches: usize,
    pub completion: SearchCompletion,
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
    BeamWidthReached,
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
    pub completion: SearchCompletion,
    pub blockers: Box<[BlueprintSynthesisBlockerCount]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrozenOperatorBlueprintSet {
    source_generation: u64,
    canonicalizer_version: u32,
    config: BlueprintBeamConfig,
    support_lineages_sha256: Box<[Commitment256]>,
    support_bundle_root_sha256: Commitment256,
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
    future_surfaces_sha256: BTreeSet<Commitment256>,
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
    pub whole_circuit_coherence_fixed: i64,
    pub covered_edges: usize,
    pub covered_planes: usize,
    pub ambiguous_bindings: usize,
    pub transform_mismatches: usize,
    pub executable_contract_mismatches: usize,
    pub eligible: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlueprintFutureEvidenceError {
    EmptyRawInput,
    InvalidExtractorVersion,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlueprintFutureEvidence {
    raw_input_sha256: Commitment256,
    extractor_version: u32,
    bundle_sha256: Commitment256,
    actor_sha256: Option<Commitment256>,
    verifier_sha256: Option<Commitment256>,
    bundle: SurfaceFragmentBundle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedBlueprintWinnerReceipt {
    source_generation: u64,
    candidate_set_sha256: Commitment256,
    support_root_sha256: Commitment256,
    future_evidence_root_sha256: Commitment256,
    future_lineage_root_sha256: Commitment256,
    evaluator_config_sha256: Commitment256,
    score_table_sha256: Commitment256,
    winner_sha256: Commitment256,
    runner_up_sha256: Commitment256,
    margin_fixed: i64,
    seal_sha256: Commitment256,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SealedBlueprintEvaluation {
    report: BlueprintFutureReport,
    winner_receipt: Option<SealedBlueprintWinnerReceipt>,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRoleMapping {
    local_to_canonical: Box<[u8]>,
    phase_fit_fixed: i64,
    phase_components_fixed: Box<[RuntimeRelationPhaseComponent]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeRelationPhaseComponent {
    plane: u8,
    source_role: u8,
    target_role: u8,
    observed_re_fixed: i32,
    observed_im_fixed: i32,
    expected_re_fixed: i32,
    expected_im_fixed: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRoleBindingReport {
    mappings: Box<[RuntimeRoleMapping]>,
    phase_winner_count: usize,
    phase_runner_up_fit_fixed: Option<i64>,
    completion: SearchCompletion,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StructuralRoleCanonicalizer;

#[derive(Clone, Copy, Debug, Default)]
pub struct BoundedRoleAligner;

#[derive(Clone, Copy, Debug, Default)]
pub struct BoundedCircuitBeam;

#[derive(Clone, Copy, Debug, Default)]
pub struct BlueprintFutureEvaluator;

#[derive(Clone, Copy, Debug, Default)]
pub struct RuntimeRoleBinder;

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
    re_fixed: i64,
    im_fixed: i64,
}

struct FutureMappingSearch<'a> {
    bundle: &'a SurfaceFragmentBundle,
    role_graph: &'a RoleGraph,
    limit: usize,
    output: Vec<Vec<u8>>,
    complete: bool,
}

impl FutureMappingSearch<'_> {
    fn visit(&mut self, local_role: usize, current: &mut Vec<u8>, used: &mut BTreeSet<u8>) {
        // Keep one look-ahead mapping so an exact-cap result cannot hide a
        // still-live search branch and masquerade as complete.
        if self.output.len() > self.limit {
            self.complete = false;
            return;
        }
        if local_role == self.bundle.roles.len() {
            self.output.push(current.clone());
            if self.output.len() > self.limit {
                self.complete = false;
            }
            return;
        }
        for (canonical_role, signature) in self.role_graph.canonical_roles.iter().enumerate() {
            let canonical_role = canonical_role as u8;
            if used.contains(&canonical_role)
                || !role_signatures_compatible(&self.bundle.roles[local_role], signature)
            {
                continue;
            }
            used.insert(canonical_role);
            current.push(canonical_role);
            self.visit(local_role + 1, current, used);
            current.pop();
            used.remove(&canonical_role);
            if self.output.len() > self.limit {
                return;
            }
        }
    }
}

impl PhaseModeAggregate {
    const SCALE: f64 = 1_000_000_000.0;

    fn add(&mut self, phase: PhaseCenterCell) {
        // Integer accumulation is commutative, so support ordering cannot alter
        // a blueprint fingerprint through floating-point rounding.
        self.re_fixed = self
            .re_fixed
            .saturating_add((phase.re * Self::SCALE).round() as i64);
        self.im_fixed = self
            .im_fixed
            .saturating_add((phase.im * Self::SCALE).round() as i64);
    }

    fn components(self) -> (f64, f64) {
        (
            self.re_fixed as f64 / Self::SCALE,
            self.im_fixed as f64 / Self::SCALE,
        )
    }
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
            if [atom.output_local_role, atom.source_a_local_role]
                .into_iter()
                .any(|role| usize::from(role) >= roles.len())
                || (atom.source_b_local_role != OPERATOR_ROLE_NONE
                    && usize::from(atom.source_b_local_role) >= roles.len())
            {
                return Err(SurfaceFragmentBundleError::InvalidLocalRole);
            }
        }
        // The high parameter byte is the canonical topological step. Sorting by
        // the derived `Ord` would put opcodes before dependencies (for example,
        // COUNT before FILTER) and turn a composed law into a different program.
        program_atoms.sort_unstable_by_key(|atom| {
            (
                atom.parameter >> 8,
                atom.opcode,
                atom.output_local_role,
                atom.source_a_local_role,
                atom.source_b_local_role,
                atom.parameter & 0x00ff,
                atom.flags,
            )
        });
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

impl BlueprintFutureEvidence {
    pub fn new(
        raw_input_sha256: Commitment256,
        extractor_version: u32,
        bundle: SurfaceFragmentBundle,
    ) -> Result<Self, BlueprintFutureEvidenceError> {
        if raw_input_sha256 == [0; 32] {
            return Err(BlueprintFutureEvidenceError::EmptyRawInput);
        }
        if extractor_version == 0 {
            return Err(BlueprintFutureEvidenceError::InvalidExtractorVersion);
        }
        let bundle_sha256 = surface_bundle_commitment(&bundle);
        Ok(Self {
            raw_input_sha256,
            extractor_version,
            bundle_sha256,
            actor_sha256: None,
            verifier_sha256: None,
            bundle,
        })
    }

    pub fn new_with_executable_contracts(
        raw_input_sha256: Commitment256,
        extractor_version: u32,
        bundle: SurfaceFragmentBundle,
        actor_sha256: Commitment256,
        verifier_sha256: Commitment256,
    ) -> Result<Self, BlueprintFutureEvidenceError> {
        let mut evidence = Self::new(raw_input_sha256, extractor_version, bundle)?;
        if actor_sha256 == [0; 32] || verifier_sha256 == [0; 32] {
            return Err(BlueprintFutureEvidenceError::EmptyRawInput);
        }
        evidence.actor_sha256 = Some(actor_sha256);
        evidence.verifier_sha256 = Some(verifier_sha256);
        Ok(evidence)
    }

    #[must_use]
    pub const fn raw_input_sha256(&self) -> &Commitment256 {
        &self.raw_input_sha256
    }

    #[must_use]
    pub const fn extractor_version(&self) -> u32 {
        self.extractor_version
    }

    #[must_use]
    pub const fn bundle_sha256(&self) -> &Commitment256 {
        &self.bundle_sha256
    }

    #[must_use]
    pub const fn bundle(&self) -> &SurfaceFragmentBundle {
        &self.bundle
    }
}

impl SealedBlueprintEvaluation {
    #[must_use]
    pub const fn report(&self) -> &BlueprintFutureReport {
        &self.report
    }

    #[must_use]
    pub const fn winner_receipt(&self) -> Option<&SealedBlueprintWinnerReceipt> {
        self.winner_receipt.as_ref()
    }
}

impl SealedBlueprintWinnerReceipt {
    #[must_use]
    pub const fn source_generation(&self) -> u64 {
        self.source_generation
    }

    #[must_use]
    pub const fn candidate_set_sha256(&self) -> &Commitment256 {
        &self.candidate_set_sha256
    }

    #[must_use]
    pub const fn support_root_sha256(&self) -> &Commitment256 {
        &self.support_root_sha256
    }

    #[must_use]
    pub const fn future_evidence_root_sha256(&self) -> &Commitment256 {
        &self.future_evidence_root_sha256
    }

    #[must_use]
    pub const fn future_lineage_root_sha256(&self) -> &Commitment256 {
        &self.future_lineage_root_sha256
    }

    #[must_use]
    pub const fn winner_sha256(&self) -> &Commitment256 {
        &self.winner_sha256
    }

    #[must_use]
    pub const fn margin_fixed(&self) -> i64 {
        self.margin_fixed
    }

    #[must_use]
    pub const fn seal_sha256(&self) -> &Commitment256 {
        &self.seal_sha256
    }

    #[must_use]
    pub fn matches_frozen(&self, frozen: &FrozenOperatorBlueprintSet) -> bool {
        self.source_generation == frozen.source_generation
            && self.candidate_set_sha256 == frozen.candidate_set_sha256
            && self.support_root_sha256 == frozen.support_bundle_root_sha256
    }

    #[must_use]
    pub fn matches_future_evidence(&self, evidence: &[BlueprintFutureEvidence]) -> bool {
        self.future_evidence_root_sha256 == future_evidence_root(evidence)
            && self.future_lineage_root_sha256 == future_lineage_root(evidence)
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
    /// Anchors one already identified semantic class in canonical role space.
    ///
    /// This does not identify a law and must not be used as transfer evidence.
    /// The caller must own a sealed singleton version-space proof; an
    /// independent future surface still has to bind and verify the circuit.
    #[must_use]
    pub fn anchor_identified_singleton(
        bundle: &SurfaceFragmentBundle,
        config: RoleAlignmentConfig,
    ) -> RoleAlignmentReport {
        if config.max_hypotheses == 0
            || config.max_hypotheses > OPERATOR_BLUEPRINT_MAX_ALIGNMENTS
            || config.max_expansions == 0
            || config.max_expansions > OPERATOR_BLUEPRINT_MAX_EXPANSIONS
            || config.color_rounds == 0
            || config.color_rounds > OPERATOR_ROLE_COLOR_ROUNDS
        {
            return blocked_alignment(RoleAlignmentBlocker::InvalidConfig);
        }
        let bindings = (0..bundle.roles.len())
            .map(|role| RoleBinding {
                bundle_index: 0,
                local_role: role as u8,
                canonical_role: role as u8,
            })
            .collect::<Vec<_>>();
        let fingerprint_sha256 = alignment_commitment(&bindings, std::slice::from_ref(bundle));
        RoleAlignmentReport {
            hypotheses: vec![RoleAlignmentHypothesis {
                bindings: bindings.into_boxed_slice(),
                canonical_role_count: bundle.roles.len() as u8,
                fingerprint_sha256,
            }]
            .into_boxed_slice(),
            expansions: 0,
            symmetric_branches: 0,
            completion: SearchCompletion::Complete { explored: 0 },
            blocker: None,
        }
    }

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
        // Full lineage commitments, not caller order, define the traversal.
        let bundle_order = canonical_bundle_order(bundles);
        let first_index = bundle_order[0];
        let first = &bundles[first_index];
        let mut states = vec![AlignmentState {
            bindings: (0..first.roles.len())
                .map(|role| RoleBinding {
                    bundle_index: first_index as u8,
                    local_role: role as u8,
                    canonical_role: role as u8,
                })
                .collect(),
            canonical_role_count: first.roles.len() as u8,
        }];
        let mut expansions = 0_usize;
        let mut symmetric_branches = 0_usize;

        for &bundle_index in &bundle_order[1..] {
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
                            if expanded.len() >= config.max_hypotheses {
                                return RoleAlignmentReport {
                                    hypotheses: Box::new([]),
                                    expansions,
                                    symmetric_branches,
                                    completion: SearchCompletion::Exhausted {
                                        stage: SearchStage::RoleAlignment,
                                        explored: expansions,
                                        frontier_remaining: 1,
                                    },
                                    blocker: Some(RoleAlignmentBlocker::BudgetExhausted),
                                };
                            }
                            expansions = expansions.saturating_add(1);
                            if expansions > config.max_expansions {
                                return RoleAlignmentReport {
                                    hypotheses: Box::new([]),
                                    expansions,
                                    symmetric_branches,
                                    completion: SearchCompletion::Exhausted {
                                        stage: SearchStage::RoleAlignment,
                                        explored: expansions,
                                        frontier_remaining: 1,
                                    },
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
                        }
                    }
                    partial =
                        deduplicate_alignment_states(expanded, bundles, config.max_hypotheses);
                    if partial.is_empty() {
                        return blocked_alignment(RoleAlignmentBlocker::NoCompatibleAlignment);
                    }
                }
                for hypothesis in partial {
                    if next.len() >= config.max_hypotheses {
                        return RoleAlignmentReport {
                            hypotheses: Box::new([]),
                            expansions,
                            symmetric_branches,
                            completion: SearchCompletion::Exhausted {
                                stage: SearchStage::RoleAlignment,
                                explored: expansions,
                                frontier_remaining: 1,
                            },
                            blocker: Some(RoleAlignmentBlocker::BudgetExhausted),
                        };
                    }
                    next.push(hypothesis);
                }
            }
            states = deduplicate_alignment_states(next, bundles, config.max_hypotheses);
        }

        let mut hypotheses = states
            .into_iter()
            .map(|mut state| {
                state.bindings.sort_unstable();
                let fingerprint_sha256 = alignment_commitment(&state.bindings, bundles);
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
            completion: SearchCompletion::Complete {
                explored: expansions,
            },
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
            source_b: if atom.source_b_local_role == OPERATOR_ROLE_NONE {
                OPERATOR_ROLE_NONE
            } else {
                self.canonical_role(bundle_index, atom.source_b_local_role)?
            },
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
        if !alignments.completion.is_complete() {
            add_blueprint_blocker(
                &mut blocker_counts,
                BlueprintSynthesisBlocker::AlignmentIncomplete,
            );
            return blueprint_report(
                Vec::new(),
                SearchCompletion::Exhausted {
                    stage: SearchStage::RoleAlignment,
                    explored: alignments.expansions,
                    frontier_remaining: 1,
                },
                blocker_counts,
            );
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
            return blueprint_report(
                Vec::new(),
                SearchCompletion::Exhausted {
                    stage: SearchStage::InputValidation,
                    explored: 0,
                    frontier_remaining: 0,
                },
                blocker_counts,
            );
        }

        let mut blueprints = BTreeMap::<Commitment256, CandidateOperatorBlueprint>::new();
        let mut expansions = 0_usize;
        let mut complete = true;
        let mut frontier_remaining = 0_usize;

        'alignments: for alignment in &alignments.hypotheses {
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
                frontier_remaining = frontier_remaining.saturating_add(1);
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
                        if expanded.len() >= config.max_blueprints {
                            complete = false;
                            frontier_remaining = frontier_remaining.saturating_add(1);
                            add_blueprint_blocker(
                                &mut blocker_counts,
                                BlueprintSynthesisBlocker::BeamWidthReached,
                            );
                            break;
                        }
                        expansions = expansions.saturating_add(1);
                        if expansions > config.max_expansions {
                            complete = false;
                            frontier_remaining = frontier_remaining.saturating_add(1);
                            add_blueprint_blocker(
                                &mut blocker_counts,
                                BlueprintSynthesisBlocker::ExpansionBudgetReached,
                            );
                            break;
                        }
                        let (aggregate_re, aggregate_im) = aggregate.components();
                        let magnitude = aggregate_re.hypot(aggregate_im);
                        if magnitude <= f64::EPSILON {
                            continue;
                        }
                        let mut branch = partial.clone();
                        branch.push(OperatorCircuitRelation {
                            cell,
                            state: mode.state,
                            phase_anchor: PhaseCenterCell {
                                re: aggregate_re / magnitude,
                                im: aggregate_im / magnitude,
                            },
                        });
                        expanded
                            .entry(relation_assignment_commitment(&branch))
                            .or_insert(branch);
                    }
                    if !complete || expansions > config.max_expansions {
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
            let Some(virtual_roles) = virtual_transform_roles(
                alignment.canonical_role_count,
                &mapped,
                &transform_program,
            ) else {
                add_blueprint_blocker(
                    &mut blocker_counts,
                    BlueprintSynthesisBlocker::TransformCapacityReached,
                );
                continue;
            };
            for mut relations in beam {
                canonicalize_phase_gauge(&mut relations);
                match OperatorCircuit::new_with_virtual_roles(
                    alignment.canonical_role_count,
                    relations,
                    &virtual_roles,
                ) {
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
                        if !blueprints.contains_key(&fingerprint_sha256)
                            && blueprints.len() >= config.max_blueprints
                        {
                            complete = false;
                            frontier_remaining = frontier_remaining.saturating_add(1);
                            add_blueprint_blocker(
                                &mut blocker_counts,
                                BlueprintSynthesisBlocker::BeamWidthReached,
                            );
                            break 'alignments;
                        }
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
        }

        if blueprints.is_empty() {
            add_blueprint_blocker(&mut blocker_counts, BlueprintSynthesisBlocker::NoBlueprint);
        }
        let completion = if complete {
            SearchCompletion::Complete {
                explored: expansions,
            }
        } else {
            SearchCompletion::Exhausted {
                stage: SearchStage::CircuitBeam,
                explored: expansions,
                frontier_remaining,
            }
        };
        blueprint_report(
            blueprints.into_values().collect(),
            completion,
            blocker_counts,
        )
    }
}

impl CandidateOperatorBlueprint {
    /// Binds the executable actor and verifier selected by the response-domain
    /// synthesizer into the blueprint identity before the candidate set is
    /// frozen. Core treats both commitments as opaque proof-carrying contracts.
    #[must_use]
    pub fn bind_executable_contracts(
        mut self,
        actor_sha256: Commitment256,
        verifier_sha256: Commitment256,
    ) -> Self {
        self.renderer_hypothesis = RendererContract {
            commitment_sha256: actor_sha256,
        };
        self.verifier_contract = VerifierContract {
            commitment_sha256: verifier_sha256,
        };
        self.fingerprint_sha256 = blueprint_commitment(
            &self.role_graph,
            &self.relation_program,
            &self.transform_program,
            &self.composition_dag,
            &self.renderer_hypothesis,
            &self.verifier_contract,
        );
        self
    }

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
    pub fn from_canonical_roles(roles: Vec<StructuralRoleSignature>) -> Option<Self> {
        if roles.is_empty() || roles.len() > OPERATOR_BLUEPRINT_MAX_ROLES {
            return None;
        }
        Some(Self {
            role_count: u8::try_from(roles.len()).ok()?,
            bindings: Box::new([]),
            canonical_roles: roles.into_boxed_slice(),
        })
    }

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
        if !report.completion.is_complete() {
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
        let mut support_bundle_commitments = bundles
            .iter()
            .map(surface_bundle_commitment)
            .collect::<Vec<_>>();
        support_bundle_commitments.sort_unstable();
        support_bundle_commitments.dedup();
        let support_bundle_root_sha256 = commitment_root(
            b"nando.blueprint-support-bundle-root.v2",
            support_bundle_commitments.into_iter(),
        );
        let candidate_set_sha256 = candidate_set_commitment(&report.blueprints);
        Ok(Self {
            source_generation,
            canonicalizer_version: OPERATOR_BLUEPRINT_CANONICALIZER_VERSION,
            config,
            support_lineages_sha256,
            support_bundle_root_sha256,
            candidate_set_sha256,
            blueprints: report.blueprints.clone(),
        })
    }

    #[must_use]
    pub fn future_window(&self) -> FrozenBlueprintFutureWindow {
        FrozenBlueprintFutureWindow {
            frozen: self.clone(),
            future_lineages_sha256: BTreeSet::new(),
            future_surfaces_sha256: BTreeSet::new(),
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
    pub const fn support_bundle_root_sha256(&self) -> &Commitment256 {
        &self.support_bundle_root_sha256
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
        self.future_surfaces_sha256.insert(bundle.surface_sha256);
        Ok(())
    }

    /// Admits a distinct future observation while retaining one phase vote per
    /// lineage. Repeated surfaces are not independent evidence.
    pub fn admit_evidence(
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
        if !self.future_surfaces_sha256.insert(bundle.surface_sha256) {
            return Err(FrozenBlueprintError::DuplicateFutureLineage);
        }
        self.future_lineages_sha256.insert(bundle.lineage_sha256);
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

    #[must_use]
    pub fn future_surfaces_sha256(&self) -> &BTreeSet<Commitment256> {
        &self.future_surfaces_sha256
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
        Self::evaluate_internal(frozen, future_bundles, None, config, control)
    }

    fn evaluate_internal(
        frozen: &FrozenOperatorBlueprintSet,
        future_bundles: &[SurfaceFragmentBundle],
        future_evidence: Option<&[BlueprintFutureEvidence]>,
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
            match window.admit_evidence(bundle) {
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
                    future_evidence,
                    &controlled_phases,
                    config,
                    control,
                )
            })
            .collect::<Vec<_>>();
        scores.sort_by(|left, right| {
            right
                .whole_circuit_coherence_fixed
                .cmp(&left.whole_circuit_coherence_fixed)
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
        let floor_fixed = coherence_fixed(config.coherence_floor);
        if best.whole_circuit_coherence_fixed < floor_fixed {
            return BlueprintFutureReport {
                control,
                scores: scores.into_boxed_slice(),
                winner_fingerprint_sha256: None,
                runner_up_margin: 0.0,
                blocker: Some(BlueprintFutureBlocker::CoherenceBelowFloor),
            };
        }
        let runner_up_fixed = scores
            .iter()
            .skip(1)
            .find(|score| score.eligible)
            .map_or(0, |score| score.whole_circuit_coherence_fixed);
        let margin_fixed = best
            .whole_circuit_coherence_fixed
            .saturating_sub(runner_up_fixed);
        let margin = margin_fixed as f64 / OPERATOR_BLUEPRINT_SCORE_SCALE as f64;
        if margin_fixed < coherence_fixed(config.coherence_margin) {
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

impl BlueprintFutureEvaluator {
    #[must_use]
    pub fn evaluate_and_seal(
        frozen: &FrozenOperatorBlueprintSet,
        future_evidence: &[BlueprintFutureEvidence],
        config: BlueprintFutureConfig,
        control: BlueprintPhaseControl,
    ) -> SealedBlueprintEvaluation {
        let bundles = future_evidence
            .iter()
            .map(|evidence| evidence.bundle.clone())
            .collect::<Vec<_>>();
        let report =
            Self::evaluate_internal(frozen, &bundles, Some(future_evidence), config, control);
        let winner_receipt = (control == BlueprintPhaseControl::Full
            && report.blocker.is_none()
            && report.winner_fingerprint_sha256.is_some())
        .then(|| seal_blueprint_winner(frozen, future_evidence, config, &report));
        SealedBlueprintEvaluation {
            report,
            winner_receipt,
        }
    }
}

impl RuntimeRoleBinder {
    #[must_use]
    pub fn bind(
        role_graph: &RoleGraph,
        relation_program: &OperatorCircuit,
        bundle: &SurfaceFragmentBundle,
        max_mappings: usize,
    ) -> RuntimeRoleBindingReport {
        if max_mappings == 0 {
            return exhausted_runtime_binding(0);
        }
        let (mappings, complete) = future_role_mappings(
            bundle,
            role_graph,
            max_mappings.min(OPERATOR_BLUEPRINT_MAX_ALIGNMENTS),
        );
        let explored = mappings.len();
        if !complete {
            return exhausted_runtime_binding(explored);
        }
        let mut valid = mappings
            .into_iter()
            .filter_map(|mapping| {
                exact_runtime_relation_fit(bundle, relation_program, &mapping).map(
                    |(phase_fit_fixed, phase_components_fixed)| RuntimeRoleMapping {
                        local_to_canonical: mapping.into_boxed_slice(),
                        phase_fit_fixed,
                        phase_components_fixed,
                    },
                )
            })
            .collect::<Vec<_>>();
        valid.sort_unstable_by(|left, right| {
            right
                .phase_fit_fixed
                .cmp(&left.phase_fit_fixed)
                .then_with(|| left.local_to_canonical.cmp(&right.local_to_canonical))
        });
        let phase_winner_count = valid.first().map_or(0, |best| {
            valid
                .iter()
                .take_while(|mapping| mapping.phase_fit_fixed == best.phase_fit_fixed)
                .count()
        });
        let phase_runner_up_fit_fixed = valid
            .get(phase_winner_count)
            .map(|mapping| mapping.phase_fit_fixed);
        RuntimeRoleBindingReport {
            mappings: valid.into_boxed_slice(),
            phase_winner_count,
            phase_runner_up_fit_fixed,
            completion: SearchCompletion::Complete { explored },
        }
    }
}

impl RuntimeRoleBindingReport {
    /// Compatibility view for callers that historically consumed only the
    /// best phase-equivalent mappings.
    #[must_use]
    pub fn mappings(&self) -> &[RuntimeRoleMapping] {
        self.phase_winner_mappings()
    }

    #[must_use]
    pub fn structural_mappings(&self) -> &[RuntimeRoleMapping] {
        &self.mappings
    }

    #[must_use]
    pub fn phase_winner_mappings(&self) -> &[RuntimeRoleMapping] {
        &self.mappings[..self.phase_winner_count]
    }

    #[must_use]
    pub const fn phase_winner_count(&self) -> usize {
        self.phase_winner_count
    }

    #[must_use]
    pub const fn phase_runner_up_fit_fixed(&self) -> Option<i64> {
        self.phase_runner_up_fit_fixed
    }

    #[must_use]
    pub fn phase_margin_fixed(&self) -> Option<i64> {
        let winner = self.mappings.first()?.phase_fit_fixed;
        self.phase_runner_up_fit_fixed
            .map(|runner_up| winner.saturating_sub(runner_up))
    }

    #[must_use]
    pub const fn completion(&self) -> SearchCompletion {
        self.completion
    }
}

impl RuntimeRoleMapping {
    #[must_use]
    pub fn local_to_canonical(&self) -> &[u8] {
        &self.local_to_canonical
    }

    #[must_use]
    pub const fn phase_fit_fixed(&self) -> i64 {
        self.phase_fit_fixed
    }

    #[must_use]
    pub fn phase_components_fixed(&self) -> &[RuntimeRelationPhaseComponent] {
        &self.phase_components_fixed
    }

    #[must_use]
    pub fn local_role_for(&self, canonical_role: u8) -> Option<u8> {
        self.local_to_canonical
            .iter()
            .position(|role| *role == canonical_role)
            .and_then(|role| u8::try_from(role).ok())
    }
}

fn exhausted_runtime_binding(explored: usize) -> RuntimeRoleBindingReport {
    RuntimeRoleBindingReport {
        mappings: Box::new([]),
        phase_winner_count: 0,
        phase_runner_up_fit_fixed: None,
        completion: SearchCompletion::Exhausted {
            stage: SearchStage::RoleAlignment,
            explored,
            frontier_remaining: 1,
        },
    }
}

fn exact_runtime_relation_fit(
    bundle: &SurfaceFragmentBundle,
    relation_program: &OperatorCircuit,
    mapping: &[u8],
) -> Option<(i64, Box<[RuntimeRelationPhaseComponent]>)> {
    if bundle.relations.len() != relation_program.relations().len() {
        return None;
    }
    let mut matched = BTreeSet::new();
    let mut phase_fit_fixed = 0_i64;
    let mut phase_components_fixed = Vec::with_capacity(bundle.relations.len());
    for observed in &bundle.relations {
        let cell = OperatorRelationCell {
            plane: observed.plane,
            source_role: *mapping.get(usize::from(observed.source_local_role))?,
            target_role: *mapping.get(usize::from(observed.target_local_role))?,
        };
        let expected_index = relation_program
            .relations()
            .iter()
            .position(|expected| expected.cell == cell && expected.state == observed.state)?;
        if !matched.insert(expected_index) {
            return None;
        }
        let expected = &relation_program.relations()[expected_index];
        let aligned = align_phase(observed.phase_anchor, expected.phase_anchor);
        phase_fit_fixed =
            phase_fit_fixed.saturating_add((aligned.re * PhaseModeAggregate::SCALE).round() as i64);
        phase_components_fixed.push(RuntimeRelationPhaseComponent {
            plane: cell.plane,
            source_role: cell.source_role,
            target_role: cell.target_role,
            observed_re_fixed: runtime_phase_fixed(observed.phase_anchor.re),
            observed_im_fixed: runtime_phase_fixed(observed.phase_anchor.im),
            expected_re_fixed: runtime_phase_fixed(expected.phase_anchor.re),
            expected_im_fixed: runtime_phase_fixed(expected.phase_anchor.im),
        });
    }
    (matched.len() == relation_program.relations().len())
        .then_some((phase_fit_fixed, phase_components_fixed.into_boxed_slice()))
}

fn runtime_phase_fixed(value: f64) -> i32 {
    (value.clamp(-1.0, 1.0) * PhaseModeAggregate::SCALE).round() as i32
}

impl RuntimeRelationPhaseComponent {
    pub const SCALE_FIXED: i64 = PhaseModeAggregate::SCALE as i64;

    #[must_use]
    pub fn try_from_fixed(
        plane: u8,
        source_role: u8,
        target_role: u8,
        observed: (i32, i32),
        expected: (i32, i32),
    ) -> Option<Self> {
        [observed.0, observed.1, expected.0, expected.1]
            .into_iter()
            .all(|value| i64::from(value).abs() <= Self::SCALE_FIXED)
            .then_some(Self {
                plane,
                source_role,
                target_role,
                observed_re_fixed: observed.0,
                observed_im_fixed: observed.1,
                expected_re_fixed: expected.0,
                expected_im_fixed: expected.1,
            })
    }

    #[must_use]
    pub const fn plane(self) -> u8 {
        self.plane
    }

    #[must_use]
    pub const fn source_role(self) -> u8 {
        self.source_role
    }

    #[must_use]
    pub const fn target_role(self) -> u8 {
        self.target_role
    }

    #[must_use]
    pub const fn observed_fixed(self) -> (i32, i32) {
        (self.observed_re_fixed, self.observed_im_fixed)
    }

    #[must_use]
    pub const fn expected_fixed(self) -> (i32, i32) {
        (self.expected_re_fixed, self.expected_im_fixed)
    }
}

fn score_blueprint_future(
    blueprint: &CandidateOperatorBlueprint,
    future_bundles: &[SurfaceFragmentBundle],
    future_evidence: Option<&[BlueprintFutureEvidence]>,
    controlled_phases: &BTreeMap<(usize, usize), Option<PhaseCenterCell>>,
    config: BlueprintFutureConfig,
    control: BlueprintPhaseControl,
) -> BlueprintFutureScore {
    let relations = blueprint.relation_program.relations();
    let mut edge_samples = vec![BTreeMap::<Commitment256, PhaseCenterCell>::new(); relations.len()];
    let mut ambiguous_bindings = 0_usize;
    let mut transform_mismatches = 0_usize;
    let executable_contract_mismatches = future_evidence.map_or(0, |evidence| {
        evidence
            .iter()
            .filter(|item| {
                item.actor_sha256.is_some_and(|actor| {
                    actor != *blueprint.renderer_hypothesis.commitment_sha256()
                }) || item.verifier_sha256.is_some_and(|verifier| {
                    verifier != *blueprint.verifier_contract.commitment_sha256()
                })
            })
            .count()
    });

    for (bundle_index, bundle) in future_bundles.iter().enumerate() {
        let (mappings, mappings_complete) = future_role_mappings(
            bundle,
            &blueprint.role_graph,
            OPERATOR_BLUEPRINT_MAX_ALIGNMENTS,
        );
        let compatible_mappings = mappings
            .into_iter()
            .filter(|mapping| {
                mapped_future_transform_program(bundle, mapping).is_some_and(|observed| {
                    observed.as_slice() == blueprint.transform_program.as_ref()
                })
            })
            .collect::<Vec<_>>();
        if mappings_complete && compatible_mappings.is_empty() {
            transform_mismatches = transform_mismatches.saturating_add(1);
            continue;
        }
        let Some(mapping) = select_future_mapping(
            &compatible_mappings,
            mappings_complete,
            bundle,
            bundle_index,
            relations,
            controlled_phases,
            control,
        ) else {
            ambiguous_bindings = ambiguous_bindings.saturating_add(1);
            continue;
        };
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
    let whole_circuit_coherence_fixed = coherence_fixed(whole_circuit_coherence);
    let required_planes = relations
        .iter()
        .map(|relation| relation.cell.plane)
        .collect::<BTreeSet<_>>()
        .len();
    let eligible = ambiguous_bindings == 0
        && transform_mismatches == 0
        && executable_contract_mismatches == 0
        && covered_edges == relations.len()
        && covered_planes == required_planes
        && whole_circuit_coherence.is_finite();

    BlueprintFutureScore {
        blueprint_fingerprint_sha256: blueprint.fingerprint_sha256,
        edge_coherences: edge_coherences.into_boxed_slice(),
        plane_coherences: plane_coherences.into_boxed_slice(),
        whole_circuit_coherence,
        whole_circuit_coherence_fixed,
        covered_edges,
        covered_planes,
        ambiguous_bindings,
        transform_mismatches,
        executable_contract_mismatches,
        eligible,
    }
}

fn mapped_future_transform_program(
    bundle: &SurfaceFragmentBundle,
    local_to_canonical: &[u8],
) -> Option<Vec<TransformOp8>> {
    let mut transforms = BTreeMap::new();
    for atom in &bundle.program_atoms {
        let map_role = |role: u8| {
            if role == OPERATOR_ROLE_NONE {
                Some(OPERATOR_ROLE_NONE)
            } else {
                local_to_canonical.get(usize::from(role)).copied()
            }
        };
        let transform = TransformOp8 {
            opcode: atom.opcode,
            output: map_role(atom.output_local_role)?,
            source_a: map_role(atom.source_a_local_role)?,
            source_b: map_role(atom.source_b_local_role)?,
            parameter: atom.parameter,
            flags: atom.flags,
        };
        transforms.entry(transform.encode()).or_insert(transform);
        if transforms.len() > OPERATOR_BLUEPRINT_MAX_PROGRAM_ATOMS {
            return None;
        }
    }
    Some(transforms.into_values().collect())
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
) -> (Vec<Vec<u8>>, bool) {
    let mut search = FutureMappingSearch {
        bundle,
        role_graph,
        limit,
        output: Vec::new(),
        complete: true,
    };
    search.visit(0, &mut Vec::new(), &mut BTreeSet::new());
    search.output.truncate(limit);
    (search.output, search.complete)
}

#[allow(clippy::too_many_arguments)]
fn select_future_mapping<'a>(
    mappings: &'a [Vec<u8>],
    mappings_complete: bool,
    bundle: &SurfaceFragmentBundle,
    bundle_index: usize,
    expected_relations: &[OperatorCircuitRelation],
    controlled_phases: &BTreeMap<(usize, usize), Option<PhaseCenterCell>>,
    control: BlueprintPhaseControl,
) -> Option<&'a Vec<u8>> {
    if !mappings_complete || mappings.is_empty() {
        return None;
    }
    let mut ranked = mappings
        .iter()
        .filter_map(|mapping| {
            let mut matched_edges = 0_usize;
            let mut phase_fit_fixed = 0_i64;
            for (relation_index, observed) in bundle.relations.iter().enumerate() {
                let sample_phase = controlled_phases
                    .get(&(bundle_index, relation_index))
                    .copied()
                    .flatten()?;
                let cell = OperatorRelationCell {
                    plane: observed.plane,
                    source_role: mapping[usize::from(observed.source_local_role)],
                    target_role: mapping[usize::from(observed.target_local_role)],
                };
                let expected = expected_relations
                    .iter()
                    .find(|expected| expected.cell == cell && expected.state == observed.state)?;
                let expected_anchor = match control {
                    BlueprintPhaseControl::MatchedRandomCenter => {
                        random_phase_anchor(expected.cell)
                    }
                    _ => expected.phase_anchor,
                };
                let aligned = align_phase(sample_phase, expected_anchor);
                phase_fit_fixed = phase_fit_fixed
                    .saturating_add((aligned.re * PhaseModeAggregate::SCALE).round() as i64);
                matched_edges = matched_edges.saturating_add(1);
            }
            (matched_edges > 0).then_some((matched_edges, phase_fit_fixed, mapping))
        })
        .collect::<Vec<_>>();
    ranked.sort_unstable_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));
    let best = ranked.first()?;
    if ranked
        .get(1)
        .is_some_and(|runner_up| (runner_up.0, runner_up.1) == (best.0, best.1))
    {
        return None;
    }
    Some(best.2)
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
            aggregate.add(relation.phase_anchor);
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

fn virtual_transform_roles(
    role_count: u8,
    mapped_relations: &BTreeMap<OperatorRelationCell, BTreeMap<PhaseModeKey, PhaseModeAggregate>>,
    transforms: &[TransformOp8],
) -> Option<Vec<u8>> {
    let observed = mapped_relations
        .keys()
        .flat_map(|cell| [cell.source_role, cell.target_role])
        .collect::<BTreeSet<_>>();
    let produced = transforms
        .iter()
        .map(|transform| transform.output)
        .collect::<BTreeSet<_>>();
    for transform in transforms {
        for source in [transform.source_a, transform.source_b] {
            if source != OPERATOR_ROLE_NONE
                && !observed.contains(&source)
                && !produced.contains(&source)
            {
                return None;
            }
        }
    }
    let virtual_roles = (0..role_count)
        .filter(|role| !observed.contains(role))
        .collect::<Vec<_>>();
    virtual_roles
        .iter()
        .all(|role| produced.contains(role))
        .then_some(virtual_roles)
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
    completion: SearchCompletion,
    blocker_counts: BTreeMap<BlueprintSynthesisBlocker, usize>,
) -> BlueprintSynthesisReport {
    blueprints.sort_by_key(|blueprint| blueprint.fingerprint_sha256);
    let expansions = match completion {
        SearchCompletion::Complete { explored } | SearchCompletion::Exhausted { explored, .. } => {
            explored
        }
    };
    BlueprintSynthesisReport {
        blueprints: blueprints.into_boxed_slice(),
        expansions,
        completion,
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

fn canonical_bundle_order(bundles: &[SurfaceFragmentBundle]) -> Vec<usize> {
    let mut order = (0..bundles.len()).collect::<Vec<_>>();
    order.sort_unstable_by_key(|index| bundles[*index].lineage_sha256);
    order
}

fn deduplicate_alignment_states(
    states: Vec<AlignmentState>,
    bundles: &[SurfaceFragmentBundle],
    limit: usize,
) -> Vec<AlignmentState> {
    let mut by_fingerprint = BTreeMap::new();
    for mut state in states {
        state.bindings.sort_unstable();
        by_fingerprint
            .entry(alignment_commitment(&state.bindings, bundles))
            .or_insert(state);
        if by_fingerprint.len() >= limit {
            break;
        }
    }
    by_fingerprint.into_values().collect()
}

fn blocked_alignment(blocker: RoleAlignmentBlocker) -> RoleAlignmentReport {
    let completion = if blocker == RoleAlignmentBlocker::NoCompatibleAlignment {
        SearchCompletion::Complete { explored: 0 }
    } else {
        SearchCompletion::Exhausted {
            stage: SearchStage::InputValidation,
            explored: 0,
            frontier_remaining: 0,
        }
    };
    RoleAlignmentReport {
        hypotheses: Box::new([]),
        expansions: 0,
        symmetric_branches: 0,
        completion,
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

fn alignment_commitment(
    bindings: &[RoleBinding],
    bundles: &[SurfaceFragmentBundle],
) -> Commitment256 {
    let mut canonical = bindings
        .iter()
        .map(|binding| {
            (
                bundles[usize::from(binding.bundle_index)].lineage_sha256,
                binding.local_role,
                binding.canonical_role,
            )
        })
        .collect::<Vec<_>>();
    canonical.sort_unstable();
    let mut hasher = Sha256::new();
    hasher.update(b"nando.role-alignment.v1");
    for (lineage, local_role, canonical_role) in canonical {
        hasher.update(lineage);
        hasher.update([local_role, canonical_role]);
    }
    hasher.finalize().into()
}

fn coherence_fixed(value: f64) -> i64 {
    if !value.is_finite() {
        return i64::MIN;
    }
    (value.clamp(-1.0, 1.0) * OPERATOR_BLUEPRINT_SCORE_SCALE as f64).round() as i64
}

fn surface_bundle_commitment(bundle: &SurfaceFragmentBundle) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.surface-fragment-bundle.v1");
    hasher.update(bundle.lineage_sha256);
    hasher.update(bundle.surface_sha256);
    hasher.update((bundle.roles.len() as u32).to_le_bytes());
    for role in &bundle.roles {
        hasher.update(role_signature_commitment(role));
    }
    hasher.update((bundle.relations.len() as u32).to_le_bytes());
    for relation in &bundle.relations {
        hasher.update([
            relation.plane,
            relation.source_local_role,
            relation.target_local_role,
            relation.state as i8 as u8,
        ]);
        hasher.update(relation.phase_anchor.re.to_bits().to_le_bytes());
        hasher.update(relation.phase_anchor.im.to_bits().to_le_bytes());
    }
    hasher.update((bundle.program_atoms.len() as u32).to_le_bytes());
    for atom in &bundle.program_atoms {
        hasher.update([
            atom.opcode,
            atom.output_local_role,
            atom.source_a_local_role,
            atom.source_b_local_role,
        ]);
        hasher.update(atom.parameter.to_le_bytes());
        hasher.update(atom.flags.to_le_bytes());
    }
    hasher.finalize().into()
}

fn future_evidence_root(evidence: &[BlueprintFutureEvidence]) -> Commitment256 {
    let mut commitments = evidence
        .iter()
        .map(|item| {
            let mut hasher = Sha256::new();
            hasher.update(b"nando.blueprint-future-evidence.v2");
            hasher.update(item.bundle.lineage_sha256);
            hasher.update(item.bundle.surface_sha256);
            hasher.update(item.bundle_sha256);
            hasher.update(item.raw_input_sha256);
            hasher.update(item.extractor_version.to_le_bytes());
            hasher.update([u8::from(item.actor_sha256.is_some())]);
            if let Some(actor_sha256) = item.actor_sha256 {
                hasher.update(actor_sha256);
            }
            hasher.update([u8::from(item.verifier_sha256.is_some())]);
            if let Some(verifier_sha256) = item.verifier_sha256 {
                hasher.update(verifier_sha256);
            }
            Commitment256::from(hasher.finalize())
        })
        .collect::<Vec<_>>();
    commitments.sort_unstable();
    commitment_root(
        b"nando.blueprint-future-evidence-root.v2",
        commitments.into_iter(),
    )
}

fn future_lineage_root(evidence: &[BlueprintFutureEvidence]) -> Commitment256 {
    commitment_root(
        b"nando.blueprint-future-lineage-root.v1",
        evidence.iter().map(|item| *item.bundle.lineage_sha256()),
    )
}

fn commitment_root(
    domain: &[u8],
    commitments: impl Iterator<Item = Commitment256>,
) -> Commitment256 {
    let mut commitments = commitments.collect::<Vec<_>>();
    commitments.sort_unstable();
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((commitments.len() as u32).to_le_bytes());
    for commitment in commitments {
        hasher.update(commitment);
    }
    hasher.finalize().into()
}

fn seal_blueprint_winner(
    frozen: &FrozenOperatorBlueprintSet,
    evidence: &[BlueprintFutureEvidence],
    config: BlueprintFutureConfig,
    report: &BlueprintFutureReport,
) -> SealedBlueprintWinnerReceipt {
    let winner_sha256 = report
        .winner_fingerprint_sha256
        .expect("sealed evaluation has a winner");
    let runner_up_sha256 = report
        .scores
        .iter()
        .find(|score| score.eligible && score.blueprint_fingerprint_sha256 != winner_sha256)
        .map_or([0; 32], |score| score.blueprint_fingerprint_sha256);
    let support_root_sha256 = frozen.support_bundle_root_sha256;
    let future_evidence_root_sha256 = future_evidence_root(evidence);
    let future_lineage_root_sha256 = future_lineage_root(evidence);
    let mut config_hasher = Sha256::new();
    config_hasher.update(b"nando.blueprint-evaluator-config.v1");
    config_hasher.update((config.min_lineages_per_edge as u64).to_le_bytes());
    config_hasher.update(coherence_fixed(config.coherence_floor).to_le_bytes());
    config_hasher.update(coherence_fixed(config.coherence_margin).to_le_bytes());
    config_hasher.update(OPERATOR_BLUEPRINT_CANONICALIZER_VERSION.to_le_bytes());
    let evaluator_config_sha256 = config_hasher.finalize().into();
    let score_table_sha256 = score_table_commitment(&report.scores);
    let margin_fixed = coherence_fixed(report.runner_up_margin);
    let mut seal_hasher = Sha256::new();
    seal_hasher.update(b"nando.sealed-blueprint-winner.v1");
    seal_hasher.update(frozen.source_generation.to_le_bytes());
    seal_hasher.update(frozen.candidate_set_sha256);
    seal_hasher.update(support_root_sha256);
    seal_hasher.update(future_evidence_root_sha256);
    seal_hasher.update(future_lineage_root_sha256);
    seal_hasher.update(evaluator_config_sha256);
    seal_hasher.update(score_table_sha256);
    seal_hasher.update(winner_sha256);
    seal_hasher.update(runner_up_sha256);
    seal_hasher.update(margin_fixed.to_le_bytes());
    let seal_sha256 = seal_hasher.finalize().into();
    SealedBlueprintWinnerReceipt {
        source_generation: frozen.source_generation,
        candidate_set_sha256: frozen.candidate_set_sha256,
        support_root_sha256,
        future_evidence_root_sha256,
        future_lineage_root_sha256,
        evaluator_config_sha256,
        score_table_sha256,
        winner_sha256,
        runner_up_sha256,
        margin_fixed,
        seal_sha256,
    }
}

fn score_table_commitment(scores: &[BlueprintFutureScore]) -> Commitment256 {
    let mut hasher = Sha256::new();
    hasher.update(b"nando.blueprint-score-table.v2");
    hasher.update((scores.len() as u32).to_le_bytes());
    for score in scores {
        hasher.update(score.blueprint_fingerprint_sha256);
        hasher.update(score.whole_circuit_coherence_fixed.to_le_bytes());
        hasher.update((score.covered_edges as u32).to_le_bytes());
        hasher.update((score.covered_planes as u32).to_le_bytes());
        hasher.update((score.ambiguous_bindings as u32).to_le_bytes());
        hasher.update((score.transform_mismatches as u32).to_le_bytes());
        hasher.update((score.executable_contract_mismatches as u32).to_le_bytes());
        hasher.update([u8::from(score.eligible)]);
        for value in score.edge_coherences.iter().chain(&score.plane_coherences) {
            hasher.update(coherence_fixed(*value).to_le_bytes());
        }
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod runtime_role_binder_tests;

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

        assert!(report.completion.is_complete());
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
        assert!(!report.completion.is_complete());
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
    fn alignment_width_truncation_is_incomplete() {
        let report = BoundedRoleAligner::align(
            &[
                symmetric_bundle(1, 11, false),
                symmetric_bundle(2, 12, true),
            ],
            RoleAlignmentConfig {
                max_hypotheses: 1,
                ..RoleAlignmentConfig::default()
            },
        );
        assert!(!report.completion.is_complete());
        assert_eq!(
            report.completion,
            SearchCompletion::Exhausted {
                stage: SearchStage::RoleAlignment,
                explored: report.expansions,
                frontier_remaining: 1,
            }
        );
        assert_eq!(report.blocker, Some(RoleAlignmentBlocker::BudgetExhausted));
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

        assert!(synthesis.completion.is_complete());
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

        let mut evidence_window = frozen.future_window();
        let same_lineage_first = symmetric_bundle(4, 14, true);
        let same_lineage_second = symmetric_bundle(4, 15, true);
        assert_eq!(evidence_window.admit_evidence(&same_lineage_first), Ok(()));
        assert_eq!(evidence_window.admit_evidence(&same_lineage_second), Ok(()));
        assert_eq!(evidence_window.future_lineages_sha256().len(), 1);
        assert_eq!(evidence_window.future_surfaces_sha256().len(), 2);
        assert_eq!(
            evidence_window.admit_evidence(&same_lineage_second),
            Err(FrozenBlueprintError::DuplicateFutureLineage)
        );
    }

    #[test]
    fn support_input_order_does_not_change_candidate_set() {
        let first = vec![
            symmetric_bundle(1, 11, false),
            symmetric_bundle(2, 12, true),
            symmetric_bundle(3, 13, false),
        ];
        let second = vec![first[2].clone(), first[0].clone(), first[1].clone()];

        let build = |bundles: &[SurfaceFragmentBundle]| {
            let alignments = BoundedRoleAligner::align(bundles, RoleAlignmentConfig::default());
            let synthesis = BoundedCircuitBeam::synthesize(
                bundles,
                &alignments,
                BlueprintBeamConfig::default(),
            );
            FrozenOperatorBlueprintSet::freeze(
                7,
                bundles,
                BlueprintBeamConfig::default(),
                &synthesis,
            )
            .expect("complete canonical candidate set")
        };

        let left = build(&first);
        let right = build(&second);
        assert_eq!(left.candidate_set_sha256(), right.candidate_set_sha256());
        assert_eq!(
            left.blueprints()
                .iter()
                .map(CandidateOperatorBlueprint::fingerprint_sha256)
                .collect::<Vec<_>>(),
            right
                .blueprints()
                .iter()
                .map(CandidateOperatorBlueprint::fingerprint_sha256)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn symmetric_local_role_renaming_does_not_change_candidate_set() {
        let original = vec![
            symmetric_bundle(1, 11, false),
            symmetric_bundle(2, 12, true),
            symmetric_bundle(3, 13, false),
        ];
        let renamed = vec![
            symmetric_bundle(1, 11, true),
            symmetric_bundle(2, 12, false),
            symmetric_bundle(3, 13, true),
        ];

        let candidate_set = |bundles: &[SurfaceFragmentBundle]| {
            let alignments = BoundedRoleAligner::align(bundles, RoleAlignmentConfig::default());
            let synthesis = BoundedCircuitBeam::synthesize(
                bundles,
                &alignments,
                BlueprintBeamConfig::default(),
            );
            FrozenOperatorBlueprintSet::freeze(
                7,
                bundles,
                BlueprintBeamConfig::default(),
                &synthesis,
            )
            .expect("complete canonical candidate set")
            .candidate_set_sha256()
            .to_owned()
        };

        assert_eq!(candidate_set(&original), candidate_set(&renamed));
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
        assert!(!report.completion.is_complete());
        assert_eq!(
            report.blockers.as_ref(),
            &[BlueprintSynthesisBlockerCount {
                blocker: BlueprintSynthesisBlocker::InvalidConfig,
                count: 1,
            }]
        );
    }

    #[test]
    fn blueprint_width_truncation_cannot_freeze() {
        let bundles = [
            symmetric_bundle(1, 11, false),
            symmetric_bundle(2, 12, true),
            symmetric_bundle(3, 13, false),
        ];
        let alignments = BoundedRoleAligner::align(&bundles, RoleAlignmentConfig::default());
        let config = BlueprintBeamConfig {
            max_blueprints: 1,
            ..BlueprintBeamConfig::default()
        };
        let report = BoundedCircuitBeam::synthesize(&bundles, &alignments, config);
        assert!(!report.completion.is_complete());
        assert!(matches!(
            report.completion,
            SearchCompletion::Exhausted {
                stage: SearchStage::CircuitBeam,
                frontier_remaining: 1..,
                ..
            }
        ));
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| { blocker.blocker == BlueprintSynthesisBlocker::BeamWidthReached })
        );
        assert_eq!(
            FrozenOperatorBlueprintSet::freeze(7, &bundles, config, &report),
            Err(FrozenBlueprintError::IncompleteSynthesis)
        );
    }
}
