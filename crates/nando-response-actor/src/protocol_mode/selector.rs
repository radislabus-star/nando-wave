use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    BindingProtocolCompilerErrorV2, BoundedProtocolModeCandidateV2, ProtocolModeCompilerBudgetV2,
    derived_mode_root_v2, is_sha256, sha256_json,
};
use crate::binding_evidence::{binding_feature_predicates_v1, binding_predicate_matches_v1};
use crate::{
    AcceptedBindingLawEvidenceV2, BindingCompletionStateV1, BindingPredicateV1,
    BindingTrialEvidenceLabelV2, BindingValueTypeV1, CanonicalEffectLawV3,
    FrozenCandidateRelationGraphV1, PhysicalTrialOutcomeV2, TrustedResolvedBindingRowV2,
};

const MAX_SELECTOR_PREDICATES_V2: usize = 3;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolRoleCardinalityV2 {
    OneActionClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolSourceRoleV2 {
    pub role_id: u16,
    pub value_type: BindingValueTypeV1,
    pub cardinality: ProtocolRoleCardinalityV2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolSourceRoleSchemaV2 {
    pub roles: Vec<ProtocolSourceRoleV2>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolSelectorProgramV2 {
    pub predicates: Vec<BindingPredicateV1>,
    pub max_action_classes: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolValueContractV2 {
    pub observed: BindingValueTypeV1,
    pub emitted: BindingValueTypeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolCapabilityContractV2 {
    pub protocol_facet_root_sha256: String,
    pub physical_program_ids_sha256: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolArgumentRoleV2 {
    pub argument_ordinal: u16,
    pub source_role_id: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolArgumentRoleSchemaV2 {
    pub roles: Vec<ProtocolArgumentRoleV2>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolConstantContractV2 {
    pub semantic_constants_sha256: Vec<String>,
    pub protocol_noop_constants_sha256: Vec<String>,
    pub execution_budget_roots_sha256: Vec<String>,
    pub transport_default_roots_sha256: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolStructuralGuardV2 {
    pub relation_identity_sha256: String,
    pub effect_invariant_root_sha256: String,
    pub selector_program_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolTemporalCardinalityContractV2 {
    pub completion_states: Vec<BindingCompletionStateV1>,
    pub temporal_distances: Vec<u16>,
    pub event_candidate_cardinalities: Vec<u16>,
    pub require_unique_action_class: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolModeProgramV2 {
    pub source_role_schema: ProtocolSourceRoleSchemaV2,
    pub selector_program: ProtocolSelectorProgramV2,
    pub value_contract: ProtocolValueContractV2,
    pub capability_contract: ProtocolCapabilityContractV2,
    pub argument_role_schema: ProtocolArgumentRoleSchemaV2,
    pub constant_contract: ProtocolConstantContractV2,
    pub structural_guard: ProtocolStructuralGuardV2,
    pub temporal_cardinality_contract: ProtocolTemporalCardinalityContractV2,
}

pub(super) struct CandidateGenerationV2 {
    pub candidates: Vec<BoundedProtocolModeCandidateV2>,
    pub search_exhausted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DerivedProtocolModeMatrixV2 {
    pub covered_positive_rows_sha256: BTreeSet<String>,
    pub accepts_negative_rows_sha256: BTreeSet<String>,
    pub wrong_action_rows_sha256: BTreeSet<String>,
    pub verify_failed_rows_sha256: BTreeSet<String>,
}

impl DerivedProtocolModeMatrixV2 {
    pub fn is_eligible(&self) -> bool {
        !self.covered_positive_rows_sha256.is_empty()
            && self.accepts_negative_rows_sha256.is_empty()
            && self.wrong_action_rows_sha256.is_empty()
            && self.verify_failed_rows_sha256.is_empty()
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ModeSeedKeyV2 {
    protocol_facet_root_sha256: String,
    effect_invariant_root_sha256: String,
    value_type: BindingValueTypeV1,
}

#[derive(Default)]
struct ModeSeedV2 {
    selector_programs: BTreeSet<Vec<BindingPredicateV1>>,
    physical_program_ids_sha256: BTreeSet<String>,
}

pub(super) fn index_frozen_graph_views_v2<'a>(
    evidence: &AcceptedBindingLawEvidenceV2,
    graph_views: &'a [FrozenCandidateRelationGraphV1],
) -> Result<BTreeMap<String, &'a FrozenCandidateRelationGraphV1>, BindingProtocolCompilerErrorV2> {
    let expected = evidence
        .rows()
        .iter()
        .map(|row| row.frozen_graph_root_sha256.clone())
        .collect::<BTreeSet<_>>();
    let mut indexed = BTreeMap::new();
    for graph in graph_views {
        // A matching digest string is not evidence; rebuild the sealed graph root.
        let refrozen = graph
            .graph
            .clone()
            .freeze()
            .map_err(|_| BindingProtocolCompilerErrorV2::InvalidGraphView)?;
        if refrozen != *graph
            || indexed
                .insert(graph.graph_root_sha256.clone(), graph)
                .is_some()
        {
            return Err(BindingProtocolCompilerErrorV2::InvalidGraphView);
        }
    }
    if indexed.keys().cloned().collect::<BTreeSet<_>>() != expected {
        return Err(BindingProtocolCompilerErrorV2::InvalidGraphView);
    }
    Ok(indexed)
}

pub(super) fn generate_protocol_mode_candidates_v2(
    evidence: &AcceptedBindingLawEvidenceV2,
    effect_law: &CanonicalEffectLawV3,
    effect_law_id_sha256: &str,
    graphs: &BTreeMap<String, &FrozenCandidateRelationGraphV1>,
    budget: ProtocolModeCompilerBudgetV2,
) -> Result<CandidateGenerationV2, BindingProtocolCompilerErrorV2> {
    let mut seeds = BTreeMap::<ModeSeedKeyV2, ModeSeedV2>::new();
    let mut search_exhausted = graphs.values().any(|graph| {
        graph.graph.extraction_budget_exhausted || graph.graph.relation_budget_exhausted
    });

    // Positive rows bound the hypothesis space, but do not decide the matrix.
    for row in evidence.rows().iter().filter(|row| {
        row.relation_identity_sha256 == evidence.relation_identity_sha256()
            && row.effect_invariant_root_sha256 == effect_law.effect_invariant_root_sha256()
            && row.evidence_label == BindingTrialEvidenceLabelV2::Positive
            && row.trial_outcome == PhysicalTrialOutcomeV2::Pass
    }) {
        let graph = graphs
            .get(&row.frozen_graph_root_sha256)
            .ok_or(BindingProtocolCompilerErrorV2::InvalidGraphView)?;
        for node in graph
            .graph
            .nodes
            .iter()
            .filter(|node| node.action_equivalence_sha256 == row.candidate_action_digest_sha256)
        {
            let key = ModeSeedKeyV2 {
                protocol_facet_root_sha256: row.protocol_facet_root_sha256.clone(),
                effect_invariant_root_sha256: row.effect_invariant_root_sha256.clone(),
                value_type: node.features.value_type,
            };
            let seed = seeds.entry(key).or_default();
            seed.physical_program_ids_sha256
                .insert(row.physical_program_id_sha256.clone());
            let atoms = binding_feature_predicates_v1(&node.features);
            add_selector_subsets_v2(
                &atoms,
                MAX_SELECTOR_PREDICATES_V2,
                budget.max_candidates,
                &mut seed.selector_programs,
                &mut search_exhausted,
            );
        }
    }

    let mut candidates = Vec::new();
    for (key, seed) in seeds {
        for predicates in seed.selector_programs {
            let source_role_schema = ProtocolSourceRoleSchemaV2 {
                roles: vec![ProtocolSourceRoleV2 {
                    role_id: 0,
                    value_type: key.value_type,
                    cardinality: ProtocolRoleCardinalityV2::OneActionClass,
                }],
            };
            let selector_program = ProtocolSelectorProgramV2 {
                predicates,
                max_action_classes: 1,
            };
            let source_role_schema_root_sha256 =
                derived_mode_root_v2("source-role-schema", &source_role_schema)?;
            let selector_program_root_sha256 =
                derived_mode_root_v2("selector-program", &selector_program)?;
            let value_contract = ProtocolValueContractV2 {
                observed: key.value_type,
                emitted: key.value_type,
            };
            let capability_contract = ProtocolCapabilityContractV2 {
                protocol_facet_root_sha256: key.protocol_facet_root_sha256.clone(),
                physical_program_ids_sha256: seed
                    .physical_program_ids_sha256
                    .iter()
                    .cloned()
                    .collect(),
            };
            let argument_role_schema = ProtocolArgumentRoleSchemaV2 {
                roles: vec![ProtocolArgumentRoleV2 {
                    argument_ordinal: 0,
                    source_role_id: 0,
                }],
            };
            let constant_contract = ProtocolConstantContractV2 {
                semantic_constants_sha256: Vec::new(),
                protocol_noop_constants_sha256: Vec::new(),
                execution_budget_roots_sha256: Vec::new(),
                transport_default_roots_sha256: Vec::new(),
            };
            let structural_guard = ProtocolStructuralGuardV2 {
                relation_identity_sha256: evidence.relation_identity_sha256().to_owned(),
                effect_invariant_root_sha256: key.effect_invariant_root_sha256.clone(),
                selector_program_root_sha256: selector_program_root_sha256.clone(),
            };
            let temporal_cardinality_contract =
                temporal_cardinality_contract_v2(&selector_program.predicates);
            let program = ProtocolModeProgramV2 {
                source_role_schema,
                selector_program,
                value_contract,
                capability_contract,
                argument_role_schema,
                constant_contract,
                structural_guard,
                temporal_cardinality_contract,
            };
            let candidate_id_sha256 = derived_mode_root_v2(
                "candidate",
                &(
                    effect_law_id_sha256,
                    evidence.relation_identity_sha256(),
                    &program,
                ),
            )?;
            candidates.push(BoundedProtocolModeCandidateV2 {
                candidate_id_sha256,
                effect_law_id_sha256: effect_law_id_sha256.to_owned(),
                relation_identity_sha256: evidence.relation_identity_sha256().to_owned(),
                protocol_facet_root_sha256: key.protocol_facet_root_sha256.clone(),
                effect_invariant_root_sha256: key.effect_invariant_root_sha256.clone(),
                source_role_schema_root_sha256,
                selector_program_root_sha256,
                observed_emitted_types_root_sha256: derived_mode_root_v2(
                    "observed-emitted-types",
                    &program.value_contract,
                )?,
                capability_protocol_root_sha256: derived_mode_root_v2(
                    "capability-protocol",
                    &program.capability_contract,
                )?,
                argument_role_schema_root_sha256: derived_mode_root_v2(
                    "argument-role-schema",
                    &program.argument_role_schema,
                )?,
                constant_contract_root_sha256: derived_mode_root_v2(
                    "constant-contract",
                    &program.constant_contract,
                )?,
                structural_guard_root_sha256: derived_mode_root_v2(
                    "structural-guard",
                    &program.structural_guard,
                )?,
                temporal_cardinality_contract_root_sha256: derived_mode_root_v2(
                    "temporal-cardinality",
                    &program.temporal_cardinality_contract,
                )?,
                action_class_root_sha256: effect_law.action_equivalence_root_sha256().to_owned(),
                program,
                covers_positive_rows_sha256: Vec::new(),
                accepts_negative_rows_sha256: Vec::new(),
                wrong_action_rows_sha256: Vec::new(),
                verify_failed_rows_sha256: Vec::new(),
                search_exhausted: false,
            });
        }
    }
    candidates.sort_by(|left, right| {
        selector_preference_key_v2(&left.program.selector_program)
            .cmp(&selector_preference_key_v2(&right.program.selector_program))
            .then_with(|| left.candidate_id_sha256.cmp(&right.candidate_id_sha256))
    });
    if candidates.len() > budget.max_candidates {
        candidates.truncate(budget.max_candidates);
        search_exhausted = true;
    }
    Ok(CandidateGenerationV2 {
        candidates,
        search_exhausted,
    })
}

pub(super) fn derive_candidate_matrix_v2(
    candidate: &BoundedProtocolModeCandidateV2,
    rows: &[TrustedResolvedBindingRowV2],
    graphs: &BTreeMap<String, &FrozenCandidateRelationGraphV1>,
    expected_action_equivalence_root_sha256: &str,
) -> Result<DerivedProtocolModeMatrixV2, BindingProtocolCompilerErrorV2> {
    let mut matrix = DerivedProtocolModeMatrixV2 {
        covered_positive_rows_sha256: BTreeSet::new(),
        accepts_negative_rows_sha256: BTreeSet::new(),
        wrong_action_rows_sha256: BTreeSet::new(),
        verify_failed_rows_sha256: BTreeSet::new(),
    };
    for row in rows.iter().filter(|row| {
        candidate.relation_identity_sha256 == row.relation_identity_sha256
            && candidate.protocol_facet_root_sha256 == row.protocol_facet_root_sha256
            && candidate.effect_invariant_root_sha256 == row.effect_invariant_root_sha256
    }) {
        let graph = graphs
            .get(&row.frozen_graph_root_sha256)
            .ok_or(BindingProtocolCompilerErrorV2::InvalidGraphView)?;
        let selected = execute_selector_v2(&candidate.program, graph)?;
        // Labels score a completed structural execution; they never select a role.
        match row.evidence_label {
            BindingTrialEvidenceLabelV2::ApplicabilityNegative => {
                if !selected.is_empty() {
                    matrix
                        .accepts_negative_rows_sha256
                        .insert(row.frozen_row_root_sha256.clone());
                }
            }
            BindingTrialEvidenceLabelV2::Positive => match row.trial_outcome {
                PhysicalTrialOutcomeV2::Pass if selected.len() == 1 => {
                    if selected.contains(&row.candidate_action_digest_sha256)
                        && candidate.action_class_root_sha256
                            == expected_action_equivalence_root_sha256
                    {
                        matrix
                            .covered_positive_rows_sha256
                            .insert(row.frozen_row_root_sha256.clone());
                    } else {
                        matrix
                            .wrong_action_rows_sha256
                            .insert(row.frozen_row_root_sha256.clone());
                    }
                }
                PhysicalTrialOutcomeV2::Pass if selected.is_empty() => {}
                PhysicalTrialOutcomeV2::Pass => {
                    matrix
                        .wrong_action_rows_sha256
                        .insert(row.frozen_row_root_sha256.clone());
                }
                PhysicalTrialOutcomeV2::Fail => {
                    if !selected.is_empty() {
                        matrix
                            .verify_failed_rows_sha256
                            .insert(row.frozen_row_root_sha256.clone());
                    }
                }
                PhysicalTrialOutcomeV2::Abstain => {
                    if !selected.is_empty() {
                        matrix
                            .wrong_action_rows_sha256
                            .insert(row.frozen_row_root_sha256.clone());
                    }
                }
                PhysicalTrialOutcomeV2::Censored => {}
            },
        }
    }
    Ok(matrix)
}

pub(super) fn validate_program_v2(
    program: &ProtocolModeProgramV2,
) -> Result<(), BindingProtocolCompilerErrorV2> {
    if program.source_role_schema.roles.len() != 1
        || program.source_role_schema.roles[0].role_id != 0
        || program.source_role_schema.roles[0].cardinality
            != ProtocolRoleCardinalityV2::OneActionClass
        || program.selector_program.max_action_classes != 1
        || program.selector_program.predicates.len() > MAX_SELECTOR_PREDICATES_V2
        || program.argument_role_schema.roles
            != vec![ProtocolArgumentRoleV2 {
                argument_ordinal: 0,
                source_role_id: 0,
            }]
        || program.value_contract.observed != program.source_role_schema.roles[0].value_type
        || program.value_contract.emitted != program.source_role_schema.roles[0].value_type
        || program.structural_guard.selector_program_root_sha256
            != derived_mode_root_v2("selector-program", &program.selector_program)?
        || !is_sha256(&program.structural_guard.relation_identity_sha256)
        || !is_sha256(&program.structural_guard.effect_invariant_root_sha256)
        || !is_sha256(&program.capability_contract.protocol_facet_root_sha256)
        || program
            .capability_contract
            .physical_program_ids_sha256
            .is_empty()
        || program
            .capability_contract
            .physical_program_ids_sha256
            .iter()
            .any(|root| !is_sha256(root))
        || constant_roots_v2(&program.constant_contract).any(|root| !is_sha256(root))
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidCandidate);
    }
    let mut predicates = program.selector_program.predicates.clone();
    predicates.sort();
    predicates.dedup();
    let mut physical_programs = program
        .capability_contract
        .physical_program_ids_sha256
        .clone();
    physical_programs.sort();
    physical_programs.dedup();
    if predicates != program.selector_program.predicates
        || physical_programs != program.capability_contract.physical_program_ids_sha256
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidCandidate);
    }
    Ok(())
}

fn execute_selector_v2(
    program: &ProtocolModeProgramV2,
    graph: &FrozenCandidateRelationGraphV1,
) -> Result<BTreeSet<String>, BindingProtocolCompilerErrorV2> {
    validate_program_v2(program)?;
    let value_type = program.source_role_schema.roles[0].value_type;
    Ok(graph
        .graph
        .nodes
        .iter()
        .filter(|node| node.features.value_type == value_type)
        .filter(|node| {
            program
                .selector_program
                .predicates
                .iter()
                .all(|predicate| binding_predicate_matches_v1(predicate, node))
        })
        .map(|node| node.action_equivalence_sha256.clone())
        .collect())
}

fn add_selector_subsets_v2(
    atoms: &[BindingPredicateV1],
    max_depth: usize,
    max_programs: usize,
    output: &mut BTreeSet<Vec<BindingPredicateV1>>,
    exhausted: &mut bool,
) {
    fn visit(
        atoms: &[BindingPredicateV1],
        start: usize,
        max_depth: usize,
        max_programs: usize,
        current: &mut Vec<BindingPredicateV1>,
        output: &mut BTreeSet<Vec<BindingPredicateV1>>,
        exhausted: &mut bool,
    ) {
        if *exhausted || current.len() == max_depth {
            return;
        }
        for index in start..atoms.len() {
            current.push(atoms[index].clone());
            output.insert(current.clone());
            if output.len() > max_programs {
                *exhausted = true;
                current.pop();
                return;
            }
            visit(
                atoms,
                index + 1,
                max_depth,
                max_programs,
                current,
                output,
                exhausted,
            );
            current.pop();
        }
    }
    output.insert(Vec::new());
    visit(
        atoms,
        0,
        max_depth,
        max_programs,
        &mut Vec::new(),
        output,
        exhausted,
    );
}

fn temporal_cardinality_contract_v2(
    predicates: &[BindingPredicateV1],
) -> ProtocolTemporalCardinalityContractV2 {
    let mut completion_states = BTreeSet::new();
    let mut temporal_distances = BTreeSet::new();
    let mut event_candidate_cardinalities = BTreeSet::new();
    for predicate in predicates {
        match predicate {
            BindingPredicateV1::CompletionState { value } => {
                completion_states.insert(*value);
            }
            BindingPredicateV1::TemporalDistance { value } => {
                temporal_distances.insert(*value);
            }
            BindingPredicateV1::EventCandidateCardinality { value } => {
                event_candidate_cardinalities.insert(*value);
            }
            _ => {}
        }
    }
    ProtocolTemporalCardinalityContractV2 {
        completion_states: completion_states.into_iter().collect(),
        temporal_distances: temporal_distances.into_iter().collect(),
        event_candidate_cardinalities: event_candidate_cardinalities.into_iter().collect(),
        require_unique_action_class: true,
    }
}

fn selector_preference_key_v2(program: &ProtocolSelectorProgramV2) -> (usize, usize, String) {
    let topology_predicates = program
        .predicates
        .iter()
        .filter(|predicate| matches!(predicate, BindingPredicateV1::TopologyNeighborhood { .. }))
        .count();
    (
        program.predicates.len(),
        topology_predicates,
        sha256_json(program).unwrap_or_default(),
    )
}

fn constant_roots_v2(contract: &ProtocolConstantContractV2) -> impl Iterator<Item = &String> {
    contract
        .semantic_constants_sha256
        .iter()
        .chain(contract.protocol_noop_constants_sha256.iter())
        .chain(contract.execution_budget_roots_sha256.iter())
        .chain(contract.transport_default_roots_sha256.iter())
}
