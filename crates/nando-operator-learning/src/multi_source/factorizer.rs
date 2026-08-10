use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceExtractionStatusV1,
    MultiSourceRelationKindV1, MultiSourceRoleNodeV1, MultiSourceTemporalClassV1,
    MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1, canonical_json_sha256,
};
use serde::{Deserialize, Serialize};

use super::{BlindThenRevealJoinedTransitionV1, CompletedEffectAtomV1};

pub const MULTI_SOURCE_FACTORIZED_ROW_SCHEMA_V1: &str = "nando.multi-source-factorized-row.v1";
pub const SOURCE_NEUTRAL_TOPOLOGY_QUOTIENT_SCHEMA_V2: &str =
    "nando.k1-source-neutral-topology-quotient.v2";
const SOURCE_NEUTRAL_TOPOLOGY_QUOTIENT_SEARCH_BUDGET_V2: usize = 4_096;

type RoleClassV2 = (
    MultiSourceTypeClassV1,
    MultiSourceContainerClassV1,
    MultiSourceCardinalityClassV1,
    MultiSourceTemporalClassV1,
    u8,
    u16,
);

type CanonicalTopologyEncodingV2 = (Vec<RoleClassV2>, Vec<(usize, usize, u8)>);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSourceReasonV1 {
    MultipleGroundedOutputs,
    MultipleOutputParts,
    SingleGroundedOutput,
    NoGroundedOutput,
    Censored,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreActionShapeClassV1 {
    SingleRoleProjection,
    OneOutputManyScalarRoles,
    ManyOutputsLatestRelevantRole,
    CrossOutputDependency,
    CollectionPlusScalarMetadata,
    MultipleCollections,
    Unresolved,
    Censored,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletedEffectFormV1 {
    SingleRoleProjection,
    MultiRoleRendering,
    StatusValueBranch,
    CollectionTransform,
    CrossOutputComposition,
    Unexplained,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FactorizedMultiSourceRowV1 {
    pub schema: String,
    pub join_root_sha256: String,
    pub turn_intent_id_sha256: String,
    pub input_tokens: u64,
    pub accepted: bool,
    pub reason: MultiSourceReasonV1,
    pub pre_action_shape: PreActionShapeClassV1,
    pub completed_effect: CompletedEffectFormV1,
    pub applicability_shape_root_sha256: String,
    pub discovery_work_root_sha256: String,
}

#[derive(Serialize)]
struct ApplicabilityShapeDigest {
    schema: &'static str,
    reason: MultiSourceReasonV1,
    pre_action_shape: PreActionShapeClassV1,
    grounded_output_count: u16,
    output_part_count: u16,
    role_signature: Vec<(String, String, String, String, u8, u16)>,
    relation_signature: Vec<String>,
}

#[must_use]
pub fn factor_multi_source_row_v1(
    joined: &BlindThenRevealJoinedTransitionV1,
) -> FactorizedMultiSourceRowV1 {
    let reason = reason(&joined.topology);
    let pre_action_shape = pre_action_shape(&joined.topology);
    let completed_effect = completed_effect(joined);
    let applicability_shape_root_sha256 = pre_action_applicability_shape_root_v1(&joined.topology)
        .expect("validated source-neutral shape serializes");
    let discovery_work_root_sha256 = canonical_json_sha256(&(
        MULTI_SOURCE_FACTORIZED_ROW_SCHEMA_V1,
        applicability_shape_root_sha256.as_str(),
        completed_effect,
        joined.semantic_action_root_sha256.as_str(),
    ))
    .expect("discovery work shape serializes");
    FactorizedMultiSourceRowV1 {
        schema: MULTI_SOURCE_FACTORIZED_ROW_SCHEMA_V1.to_owned(),
        join_root_sha256: joined.join_root_sha256.clone(),
        turn_intent_id_sha256: joined.turn_intent_id_sha256.clone(),
        input_tokens: joined.input_tokens,
        accepted: joined.accepted,
        reason,
        pre_action_shape,
        completed_effect,
        applicability_shape_root_sha256,
        discovery_work_root_sha256,
    }
}

pub fn pre_action_applicability_shape_root_v1(
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<String, &'static str> {
    topology.validate()?;
    let applicability = ApplicabilityShapeDigest {
        schema: MULTI_SOURCE_FACTORIZED_ROW_SCHEMA_V1,
        reason: reason(topology),
        pre_action_shape: pre_action_shape(topology),
        grounded_output_count: topology.grounded_output_count,
        output_part_count: topology.output_part_count,
        role_signature: topology
            .roles
            .iter()
            .map(|role| {
                (
                    format!("{:?}", role.type_class),
                    format!("{:?}", role.container_class),
                    format!("{:?}", role.cardinality_class),
                    format!("{:?}", role.temporal_class),
                    role.depth_bucket,
                    role.structural_flags,
                )
            })
            .collect(),
        relation_signature: topology
            .relations
            .iter()
            .map(|edge| format!("{:?}", edge.relation))
            .collect(),
    };
    canonical_json_sha256(&applicability)
}

pub fn source_neutral_topology_root_v1(
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<String, &'static str> {
    topology.validate()?;
    let role_index = topology
        .roles
        .iter()
        .enumerate()
        .map(|(index, role)| (role.local_role_id, index))
        .collect::<BTreeMap<_, _>>();
    let roles = topology
        .roles
        .iter()
        .map(|role| {
            (
                role.type_class,
                role.container_class,
                role.cardinality_class,
                role.temporal_class,
                role.depth_bucket,
                role.structural_flags,
            )
        })
        .collect::<Vec<_>>();
    let relations = topology
        .relations
        .iter()
        .map(|edge| {
            Ok((
                edge.relation,
                *role_index
                    .get(&edge.source_role_id)
                    .ok_or("source_neutral_topology_source_role_missing")?,
                *role_index
                    .get(&edge.target_role_id)
                    .ok_or("source_neutral_topology_target_role_missing")?,
            ))
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    canonical_json_sha256(&(
        "nando.k1-source-neutral-role-graph.v1",
        topology.grounded_output_count,
        topology.output_part_count,
        roles,
        relations,
    ))
}

/// Canonicalizes role graphs up to permutations of structurally indistinguishable roles.
pub fn source_neutral_topology_quotient_root_v2(
    topology: &PreActionMultiSourceTopologyV1,
) -> Result<String, &'static str> {
    topology.validate()?;
    let role_classes = topology.roles.iter().map(role_class_v2).collect::<Vec<_>>();
    let role_index = topology
        .roles
        .iter()
        .enumerate()
        .map(|(index, role)| (role.local_role_id, index))
        .collect::<BTreeMap<_, _>>();
    let mut relation_masks = vec![vec![0u8; topology.roles.len()]; topology.roles.len()];
    for edge in &topology.relations {
        let source = *role_index
            .get(&edge.source_role_id)
            .ok_or("source_neutral_topology_source_role_missing")?;
        let target = *role_index
            .get(&edge.target_role_id)
            .ok_or("source_neutral_topology_target_role_missing")?;
        relation_masks[source][target] |= relation_bit_v2(edge.relation);
    }

    let mut distinct_classes = role_classes.clone();
    distinct_classes.sort();
    distinct_classes.dedup();
    let initial_colors = role_classes
        .iter()
        .map(|class| {
            distinct_classes
                .binary_search(class)
                .map_err(|_| "source_neutral_topology_role_class_missing")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut canonicalizer = TopologyCanonicalizerV2 {
        role_classes: &role_classes,
        relation_masks: &relation_masks,
        search_nodes: 0,
        best: None,
    };
    if canonicalizer.search(initial_colors) {
        let (roles, relations) = canonicalizer
            .best
            .ok_or("source_neutral_topology_canonicalization_empty")?;
        return canonical_json_sha256(&(
            SOURCE_NEUTRAL_TOPOLOGY_QUOTIENT_SCHEMA_V2,
            "exact_permutation_quotient",
            topology.grounded_output_count,
            topology.output_part_count,
            roles,
            relations,
        ));
    }

    // Budget exhaustion must lose grouping power, never merge distinct topologies.
    canonical_json_sha256(&(
        SOURCE_NEUTRAL_TOPOLOGY_QUOTIENT_SCHEMA_V2,
        "exact_role_order_fallback",
        source_neutral_topology_root_v1(topology)?,
    ))
}

struct TopologyCanonicalizerV2<'a> {
    role_classes: &'a [RoleClassV2],
    relation_masks: &'a [Vec<u8>],
    search_nodes: usize,
    best: Option<CanonicalTopologyEncodingV2>,
}

impl TopologyCanonicalizerV2<'_> {
    fn search(&mut self, colors: Vec<usize>) -> bool {
        self.search_nodes = self.search_nodes.saturating_add(1);
        if self.search_nodes > SOURCE_NEUTRAL_TOPOLOGY_QUOTIENT_SEARCH_BUDGET_V2 {
            return false;
        }
        let colors = self.refine(colors);
        let mut cells = BTreeMap::<usize, Vec<usize>>::new();
        for (role, color) in colors.iter().copied().enumerate() {
            cells.entry(color).or_default().push(role);
        }
        let Some(cell) = cells
            .iter()
            .filter(|(_, roles)| roles.len() > 1)
            .min_by_key(|(color, roles)| (roles.len(), **color))
            .map(|(_, roles)| roles.clone())
        else {
            let mut order = (0..colors.len()).collect::<Vec<_>>();
            order.sort_by_key(|role| colors[*role]);
            let encoding = self.encoding(&order);
            if self.best.as_ref().is_none_or(|best| encoding < *best) {
                self.best = Some(encoding);
            }
            return true;
        };

        let mut representatives = Vec::new();
        for role in cell {
            if representatives
                .iter()
                .any(|other| self.transposition_is_automorphism(*other, role))
            {
                continue;
            }
            representatives.push(role);
            let mut individualized = colors.clone();
            individualized[role] = colors.iter().copied().max().unwrap_or(0) + 1;
            if !self.search(individualized) {
                return false;
            }
        }
        true
    }

    fn refine(&self, mut colors: Vec<usize>) -> Vec<usize> {
        loop {
            let signatures = (0..colors.len())
                .map(|role| {
                    let mut neighborhood = Vec::new();
                    for (other, other_color) in colors.iter().copied().enumerate() {
                        let outgoing = self.relation_masks[role][other];
                        if outgoing != 0 {
                            neighborhood.push((0u8, outgoing, other_color));
                        }
                        let incoming = self.relation_masks[other][role];
                        if incoming != 0 {
                            neighborhood.push((1u8, incoming, other_color));
                        }
                    }
                    neighborhood.sort();
                    (colors[role], neighborhood)
                })
                .collect::<Vec<_>>();
            let mut distinct = signatures.clone();
            distinct.sort();
            distinct.dedup();
            let next = signatures
                .iter()
                .map(|signature| {
                    distinct
                        .binary_search(signature)
                        .expect("refinement signature must exist")
                })
                .collect::<Vec<_>>();
            if next == colors {
                return colors;
            }
            colors = next;
        }
    }

    fn transposition_is_automorphism(&self, left: usize, right: usize) -> bool {
        if self.role_classes[left] != self.role_classes[right] {
            return false;
        }
        let swap = |role| {
            if role == left {
                right
            } else if role == right {
                left
            } else {
                role
            }
        };
        (0..self.role_classes.len()).all(|source| {
            (0..self.role_classes.len()).all(|target| {
                self.relation_masks[source][target]
                    == self.relation_masks[swap(source)][swap(target)]
            })
        })
    }

    fn encoding(&self, order: &[usize]) -> CanonicalTopologyEncodingV2 {
        let roles = order
            .iter()
            .map(|role| self.role_classes[*role])
            .collect::<Vec<_>>();
        let mut relations = Vec::new();
        for (source, source_role) in order.iter().copied().enumerate() {
            for (target, target_role) in order.iter().copied().enumerate() {
                let mask = self.relation_masks[source_role][target_role];
                if mask != 0 {
                    relations.push((source, target, mask));
                }
            }
        }
        (roles, relations)
    }
}

fn relation_bit_v2(relation: MultiSourceRelationKindV1) -> u8 {
    match relation {
        MultiSourceRelationKindV1::Contains => 1 << 0,
        MultiSourceRelationKindV1::Precedes => 1 << 1,
        MultiSourceRelationKindV1::SameOutput => 1 << 2,
        MultiSourceRelationKindV1::LatestOutput => 1 << 3,
        MultiSourceRelationKindV1::ContinuationHandle => 1 << 4,
        MultiSourceRelationKindV1::RequestReferencesRole => 1 << 5,
        MultiSourceRelationKindV1::CapabilityPermitsRole => 1 << 6,
    }
}

fn role_class_v2(role: &MultiSourceRoleNodeV1) -> RoleClassV2 {
    (
        role.type_class,
        role.container_class,
        role.cardinality_class,
        role.temporal_class,
        role.depth_bucket,
        role.structural_flags,
    )
}

fn reason(topology: &PreActionMultiSourceTopologyV1) -> MultiSourceReasonV1 {
    if matches!(
        topology.extraction_status,
        MultiSourceExtractionStatusV1::Censored { .. }
    ) {
        MultiSourceReasonV1::Censored
    } else if topology.grounded_output_count > 1 {
        MultiSourceReasonV1::MultipleGroundedOutputs
    } else if topology.output_part_count > 1 {
        MultiSourceReasonV1::MultipleOutputParts
    } else if topology.grounded_output_count == 1 {
        MultiSourceReasonV1::SingleGroundedOutput
    } else {
        MultiSourceReasonV1::NoGroundedOutput
    }
}

fn pre_action_shape(topology: &PreActionMultiSourceTopologyV1) -> PreActionShapeClassV1 {
    if matches!(
        topology.extraction_status,
        MultiSourceExtractionStatusV1::Censored { .. }
    ) {
        return PreActionShapeClassV1::Censored;
    }
    let collections = topology
        .roles
        .iter()
        .filter(|role| !matches!(role.container_class, MultiSourceContainerClassV1::Scalar))
        .count();
    let scalars = topology.roles.len().saturating_sub(collections);
    if collections > 1 {
        return PreActionShapeClassV1::MultipleCollections;
    }
    if collections == 1 && scalars > 0 {
        return PreActionShapeClassV1::CollectionPlusScalarMetadata;
    }
    let source_ordinals = topology
        .roles
        .iter()
        .map(|role| role.source_ordinal)
        .collect::<BTreeSet<_>>();
    let cross_output_relation = topology.relations.iter().any(|edge| {
        let source = topology
            .roles
            .iter()
            .find(|role| role.local_role_id == edge.source_role_id);
        let target = topology
            .roles
            .iter()
            .find(|role| role.local_role_id == edge.target_role_id);
        !matches!(edge.relation, MultiSourceRelationKindV1::Precedes)
            && source
                .zip(target)
                .is_some_and(|(left, right)| left.source_ordinal != right.source_ordinal)
    });
    if cross_output_relation {
        return PreActionShapeClassV1::CrossOutputDependency;
    }
    if source_ordinals.len() > 1
        && topology
            .roles
            .iter()
            .any(|role| role.temporal_class == MultiSourceTemporalClassV1::Latest)
    {
        return PreActionShapeClassV1::ManyOutputsLatestRelevantRole;
    }
    match (topology.grounded_output_count, topology.roles.len()) {
        (1, 1) => PreActionShapeClassV1::SingleRoleProjection,
        (1, count) if count > 1 => PreActionShapeClassV1::OneOutputManyScalarRoles,
        _ => PreActionShapeClassV1::Unresolved,
    }
}

fn completed_effect(joined: &BlindThenRevealJoinedTransitionV1) -> CompletedEffectFormV1 {
    let atoms = &joined.effect_atoms;
    if atoms.contains(&CompletedEffectAtomV1::StatusProjection) {
        CompletedEffectFormV1::StatusValueBranch
    } else if atoms.contains(&CompletedEffectAtomV1::ValueProjection)
        && joined
            .topology
            .roles
            .iter()
            .any(|role| !matches!(role.container_class, MultiSourceContainerClassV1::Scalar))
    {
        CompletedEffectFormV1::CollectionTransform
    } else {
        let legacy_role_input = atoms.contains(&CompletedEffectAtomV1::RoleInput);
        let role_input_slots = atoms
            .iter()
            .filter_map(|atom| match atom {
                CompletedEffectAtomV1::RoleInputSlot { slot_id, .. } => Some(*slot_id),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let role_inputs = role_input_slots.len().max(usize::from(legacy_role_input));
        match role_inputs {
            0 => CompletedEffectFormV1::Unexplained,
            1 => CompletedEffectFormV1::SingleRoleProjection,
            _ if joined.topology.grounded_output_count > 1 => {
                CompletedEffectFormV1::CrossOutputComposition
            }
            _ => CompletedEffectFormV1::MultiRoleRendering,
        }
    }
}

#[cfg(test)]
mod tests {
    use nando_operator_kernel::{
        MultiSourceExtractionStatusV1, MultiSourceRelationEdgeV1, MultiSourceRelationKindV1,
        MultiSourceRoleNodeV1,
    };

    use super::*;

    fn role(local_role_id: u16) -> MultiSourceRoleNodeV1 {
        MultiSourceRoleNodeV1 {
            local_role_id,
            source_ordinal: local_role_id,
            value_ordinal: 0,
            type_class: MultiSourceTypeClassV1::String,
            container_class: MultiSourceContainerClassV1::Scalar,
            cardinality_class: MultiSourceCardinalityClassV1::One,
            temporal_class: MultiSourceTemporalClassV1::Historical,
            depth_bucket: 1,
            structural_flags: 0,
        }
    }

    fn topology(role_count: u16, edges: &[(u16, u16)]) -> PreActionMultiSourceTopologyV1 {
        let mut relations = edges
            .iter()
            .map(
                |(source_role_id, target_role_id)| MultiSourceRelationEdgeV1 {
                    relation: MultiSourceRelationKindV1::Precedes,
                    source_role_id: *source_role_id,
                    target_role_id: *target_role_id,
                },
            )
            .collect::<Vec<_>>();
        relations.sort();
        PreActionMultiSourceTopologyV1 {
            extraction_status: MultiSourceExtractionStatusV1::Complete,
            grounded_output_count: 1,
            output_part_count: 1,
            roles: (0..role_count).map(role).collect(),
            role_witnesses: Vec::new(),
            relations,
        }
    }

    #[test]
    fn topology_quotient_collapses_only_role_id_permutations() {
        let chain = topology(3, &[(0, 1), (1, 2)]);
        let permuted_chain = topology(3, &[(2, 1), (1, 0)]);
        let star = topology(3, &[(0, 1), (0, 2)]);

        assert_ne!(
            source_neutral_topology_root_v1(&chain).expect("exact chain root"),
            source_neutral_topology_root_v1(&permuted_chain).expect("exact permuted root")
        );
        assert_eq!(
            source_neutral_topology_quotient_root_v2(&chain).expect("quotient chain root"),
            source_neutral_topology_quotient_root_v2(&permuted_chain)
                .expect("quotient permuted root")
        );
        assert_ne!(
            source_neutral_topology_quotient_root_v2(&chain).expect("quotient chain root"),
            source_neutral_topology_quotient_root_v2(&star).expect("quotient star root")
        );
    }

    #[test]
    fn topology_quotient_separates_regular_non_isomorphic_graphs() {
        let cycle = topology(6, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0)]);
        let two_cycles = topology(6, &[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)]);

        assert_ne!(
            source_neutral_topology_quotient_root_v2(&cycle).expect("six-cycle quotient"),
            source_neutral_topology_quotient_root_v2(&two_cycles)
                .expect("two three-cycles quotient")
        );
    }
}
