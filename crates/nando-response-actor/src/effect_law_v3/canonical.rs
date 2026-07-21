use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use super::*;
use crate::{RelationAtom, canonical_json_bytes, valid_nonzero_sha256};

#[derive(Clone)]
struct CanonicalizationCandidate {
    topology_bytes: Vec<u8>,
    law_bytes: Vec<u8>,
    law: CanonicalEffectLawV3,
    mapping: Vec<CanonicalNodeMappingV3>,
    typed_constants_root_sha256: String,
    completion_status_renderer_root_sha256: String,
    temporal_cardinality_root_sha256: String,
}

pub(super) struct ObservationClassificationFacetsV3 {
    pub law: CanonicalEffectLawV3,
    pub grouping_key_sha256: String,
    pub typed_constants_root_sha256: String,
    pub completion_status_renderer_root_sha256: String,
    pub temporal_cardinality_root_sha256: String,
}

type NodeColor = (EffectSource, u16, u16, bool, Option<u16>);

pub(super) fn search_quotient(
    observations: &[SealedEffectObservationV3],
    dictionary: &EffectLawDictionaryV3,
    hypothesis: &EffectQuotientHypothesisV3,
) -> Result<EffectLawQuotientReportV3, EffectLawV3Error> {
    validate_dictionary(dictionary)?;
    validate_hypothesis(hypothesis)?;
    if observations.len() < 2 || observations.len() > MAX_OBSERVATIONS_V3 {
        return Err(EffectLawV3Error::InsufficientIndependentEvidence);
    }
    for observation in observations {
        evidence::validate_sealed_observation(observation)?;
    }
    let unique_observations = observations
        .iter()
        .map(|item| item.observation_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let episode_lineages = observations
        .iter()
        .map(|item| item.episode_lineage_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let surface_roots = observations
        .iter()
        .map(|item| item.surface_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let physical_program_ids = observations
        .iter()
        .map(|item| item.physical_program_id.as_str())
        .collect::<BTreeSet<_>>();
    let trust_manifest_roots = observations
        .iter()
        .map(|item| item.trust_manifest_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let resolver_roots = observations
        .iter()
        .map(|item| item.resolver_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let delta_verifier_roots = observations
        .iter()
        .map(|item| item.delta_verifier_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let independence = EffectLawIndependenceV3 {
        observations: unique_observations.len(),
        episode_lineages: episode_lineages.len(),
        surface_roots: surface_roots.len(),
        physical_program_ids: physical_program_ids.len(),
    };
    if independence.observations != observations.len()
        || independence.episode_lineages < 2
        || independence.surface_roots < 2
        || independence.physical_program_ids < 2
        || trust_manifest_roots.len() != 1
        || resolver_roots.len() != 1
        || delta_verifier_roots.len() != 1
    {
        return Err(EffectLawV3Error::InsufficientIndependentEvidence);
    }
    let mut observation_ids = unique_observations
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    observation_ids.sort();
    let observation_set_root_sha256 = evidence::sha256_serialized(&observation_ids)?;

    let mut canonicalized = Vec::with_capacity(observations.len());
    for observation in observations {
        canonicalized.push(canonicalize_observation(
            observation,
            dictionary,
            hypothesis,
        )?);
    }
    let reference = canonicalized[0].law.canonical_bytes()?;
    if canonicalized.iter().skip(1).any(|item| {
        !item
            .law
            .canonical_bytes()
            .is_ok_and(|bytes| bytes == reference)
    }) {
        return Ok(EffectLawQuotientReportV3 {
            independence,
            observation_set_root_sha256,
            candidate: None,
            blocker: Some("no_invariant_effect_delta".to_owned()),
        });
    }
    let law = canonicalized[0].law.clone();
    let mut proofs = canonicalized
        .into_iter()
        .zip(observations)
        .map(|(canonical, observation)| ObservationCanonicalProofV3 {
            observation_sha256: observation.observation_sha256.clone(),
            evidence_ref_sha256: observation.evidence_ref_sha256.clone(),
            transition_sha256: observation.transition_sha256.clone(),
            episode_lineage_sha256: observation.episode_lineage_sha256.clone(),
            surface_root_sha256: observation.surface_root_sha256.clone(),
            physical_program_id: observation.physical_program_id.clone(),
            node_mapping: canonical.mapping,
            exact_delta_root_sha256: observation.delta.exact_root_sha256.clone(),
            capture_receipt_root_sha256: observation.capture_receipt_root_sha256.clone(),
            parity_receipt_root_sha256: observation.parity_receipt_root_sha256.clone(),
            verifier_root_sha256: observation.verifier_root_sha256.clone(),
            resolver_root_sha256: observation.resolver_root_sha256.clone(),
            trust_manifest_root_sha256: observation.trust_manifest_root_sha256.clone(),
            observed_state_root_sha256: observation.observed_state_root_sha256.clone(),
            verified_delta_receipt_root_sha256: observation
                .verified_delta_receipt_root_sha256
                .clone(),
            delta_verifier_root_sha256: observation.delta_verifier_root_sha256.clone(),
        })
        .collect::<Vec<_>>();
    proofs.sort_by(|left, right| left.observation_sha256.cmp(&right.observation_sha256));
    let proof_set_root_sha256 = evidence::sha256_serialized(&proofs)?;
    let bundle_sha256 = restart_bundle_digest(&law, &proofs, &proof_set_root_sha256)?;
    let restart_bundle = EffectLawRestartBundleV3 {
        schema: EFFECT_LAW_RESTART_BUNDLE_SCHEMA_V3.to_owned(),
        law: law.clone(),
        proofs,
        proof_set_root_sha256,
        bundle_sha256,
    };
    Ok(EffectLawQuotientReportV3 {
        independence,
        observation_set_root_sha256,
        candidate: Some(CanonicalEffectLawCandidateV3 {
            law,
            restart_bundle,
        }),
        blocker: None,
    })
}

pub(super) fn observation_classification_facets(
    observation: &SealedEffectObservationV3,
    dictionary: &EffectLawDictionaryV3,
    hypothesis: &EffectQuotientHypothesisV3,
) -> Result<ObservationClassificationFacetsV3, EffectLawV3Error> {
    validate_dictionary(dictionary)?;
    validate_hypothesis(hypothesis)?;
    evidence::validate_sealed_observation(observation)?;
    let canonical = canonicalize_observation(observation, dictionary, hypothesis)?;
    Ok(ObservationClassificationFacetsV3 {
        grouping_key_sha256: evidence::sha256_serialized(&canonical.law)?,
        law: canonical.law,
        typed_constants_root_sha256: canonical.typed_constants_root_sha256,
        completion_status_renderer_root_sha256: canonical.completion_status_renderer_root_sha256,
        temporal_cardinality_root_sha256: canonical.temporal_cardinality_root_sha256,
    })
}

fn validate_dictionary(dictionary: &EffectLawDictionaryV3) -> Result<(), EffectLawV3Error> {
    if dictionary.schema != "nando.effect-law-dictionary.v3"
        || evidence::build_dictionary(dictionary.version, dictionary.entries.clone())?.root_sha256
            != dictionary.root_sha256
    {
        return Err(EffectLawV3Error::InvalidDictionary);
    }
    Ok(())
}

fn validate_hypothesis(hypothesis: &EffectQuotientHypothesisV3) -> Result<(), EffectLawV3Error> {
    if hypothesis.schema != EFFECT_QUOTIENT_HYPOTHESIS_SCHEMA_V3
        || hypothesis.version != EFFECT_LAW_IR_VERSION_V3
        || hypothesis.projected_atom_classes != [EFFECT_ATOM_PHYSICAL_SURFACE]
        || hypothesis.root_sha256
            != evidence::sha256_serialized(&(
                hypothesis.schema.as_str(),
                hypothesis.version,
                &hypothesis.projected_atom_classes,
            ))?
    {
        return Err(EffectLawV3Error::NoInvariantQuotient);
    }
    Ok(())
}

fn canonicalize_observation(
    observation: &SealedEffectObservationV3,
    dictionary: &EffectLawDictionaryV3,
    hypothesis: &EffectQuotientHypothesisV3,
) -> Result<CanonicalizationCandidate, EffectLawV3Error> {
    let graph = &observation.physical_graph;
    if graph.nodes.is_empty()
        || graph.nodes.len() > MAX_EFFECT_NODES_V3
        || graph.edges.len() > MAX_EFFECT_EDGES_V3
    {
        return Err(EffectLawV3Error::OverBudget);
    }
    let mut groups = BTreeMap::<NodeColor, Vec<usize>>::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        groups
            .entry((
                node.source,
                node.node_kind_code,
                node.value_type_code,
                node.unique,
                node.operation_code,
            ))
            .or_default()
            .push(index);
    }
    let groups = groups.into_values().collect::<Vec<_>>();
    let permutations = groups.iter().try_fold(1_usize, |total, group| {
        total.checked_mul(factorial(group.len())?)
    });
    if permutations.is_none_or(|count| count > MAX_CANONICAL_PERMUTATIONS_V3) {
        return Err(EffectLawV3Error::OverBudget);
    }
    let mut ordered = Vec::with_capacity(graph.nodes.len());
    let mut candidates = Vec::new();
    enumerate_groups(
        observation,
        dictionary,
        hypothesis,
        &groups,
        0,
        &mut ordered,
        &mut candidates,
    )?;
    candidates.sort_by(|left, right| left.law_bytes.cmp(&right.law_bytes));
    let selected = candidates
        .first()
        .cloned()
        .ok_or(EffectLawV3Error::IncompleteEffectDelta)?;
    let action_classes = candidates
        .iter()
        .filter(|candidate| candidate.topology_bytes == selected.topology_bytes)
        .map(|candidate| candidate.law.action_equivalence_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let binding_classes = candidates
        .iter()
        .filter(|candidate| candidate.topology_bytes == selected.topology_bytes)
        .map(|candidate| {
            let mapping = candidate
                .mapping
                .iter()
                .map(|item| (item.physical_node, item.canonical_node))
                .collect::<BTreeMap<_, _>>();
            observation
                .role_bindings
                .iter()
                .map(|binding| mapped(&mapping, binding.physical_node))
                .collect::<Result<Vec<_>, EffectLawV3Error>>()
        })
        .collect::<Result<BTreeSet<_>, EffectLawV3Error>>()?;
    if action_classes.len() != 1
        || (observation.role_bindings.len() > 1 && binding_classes.len() != 1)
    {
        return Err(EffectLawV3Error::AmbiguousActionEquivalence);
    }
    Ok(selected)
}

#[allow(clippy::too_many_arguments)]
fn enumerate_groups(
    observation: &SealedEffectObservationV3,
    dictionary: &EffectLawDictionaryV3,
    hypothesis: &EffectQuotientHypothesisV3,
    groups: &[Vec<usize>],
    group_index: usize,
    ordered: &mut Vec<usize>,
    output: &mut Vec<CanonicalizationCandidate>,
) -> Result<(), EffectLawV3Error> {
    if group_index < groups.len() {
        let mut group = groups[group_index].clone();
        return enumerate_permutations(&mut group, 0, &mut |permutation| {
            ordered.extend_from_slice(permutation);
            enumerate_groups(
                observation,
                dictionary,
                hypothesis,
                groups,
                group_index + 1,
                ordered,
                output,
            )?;
            ordered.truncate(ordered.len().saturating_sub(permutation.len()));
            Ok(())
        });
    }
    output.push(build_candidate(
        observation,
        dictionary,
        hypothesis,
        ordered,
    )?);
    Ok(())
}

fn enumerate_permutations(
    values: &mut [usize],
    index: usize,
    visit: &mut impl FnMut(&[usize]) -> Result<(), EffectLawV3Error>,
) -> Result<(), EffectLawV3Error> {
    if index == values.len() {
        return visit(values);
    }
    for candidate in index..values.len() {
        values.swap(index, candidate);
        enumerate_permutations(values, index + 1, visit)?;
        values.swap(index, candidate);
    }
    Ok(())
}

fn factorial(value: usize) -> Option<usize> {
    (2..=value).try_fold(1_usize, |product, next| product.checked_mul(next))
}

fn build_candidate(
    observation: &SealedEffectObservationV3,
    dictionary: &EffectLawDictionaryV3,
    hypothesis: &EffectQuotientHypothesisV3,
    ordered: &[usize],
) -> Result<CanonicalizationCandidate, EffectLawV3Error> {
    let graph = &observation.physical_graph;
    let mut physical_to_canonical = BTreeMap::new();
    let topology_nodes = ordered
        .iter()
        .enumerate()
        .map(|(canonical, physical_index)| {
            let node = &graph.nodes[*physical_index];
            let canonical_node =
                u16::try_from(canonical).map_err(|_| EffectLawV3Error::OverBudget)?;
            physical_to_canonical.insert(node.physical_node, canonical_node);
            Ok(CanonicalEffectNodeV3 {
                canonical_node,
                source: node.source,
                node_kind_code: node.node_kind_code,
                value_type_code: node.value_type_code,
                unique: node.unique,
                operation_code: node.operation_code,
            })
        })
        .collect::<Result<Vec<_>, EffectLawV3Error>>()?;
    let mut topology_edges = graph
        .edges
        .iter()
        .map(|edge| {
            Ok(CanonicalEffectEdgeV3 {
                from: mapped(&physical_to_canonical, edge.from)?,
                to: mapped(&physical_to_canonical, edge.to)?,
                relation_code: edge.relation_code,
            })
        })
        .collect::<Result<Vec<_>, EffectLawV3Error>>()?;
    topology_edges.sort();
    let topology_bytes = canonical_json_bytes(&(&topology_nodes, &topology_edges))
        .map_err(|_| EffectLawV3Error::Serialization)?;

    let mut relation_program = topology_edges
        .iter()
        .map(|edge| CanonicalRelationClauseV3 {
            relation_code: edge.relation_code,
            lhs: edge.from,
            rhs: Some(edge.to),
            argument_ordinal: None,
            constant_type_code: None,
            constant_sha256: None,
        })
        .collect::<Vec<_>>();
    relation_program.extend(topology_nodes.iter().map(|node| CanonicalRelationClauseV3 {
        relation_code: EFFECT_REL_REQUIRE,
        lhs: node.canonical_node,
        rhs: None,
        argument_ordinal: None,
        constant_type_code: None,
        constant_sha256: None,
    }));
    let mut canonical_bindings = observation
        .role_bindings
        .iter()
        .map(|binding| {
            Ok((
                mapped(&physical_to_canonical, binding.physical_node)?,
                binding.value_type_code,
            ))
        })
        .collect::<Result<Vec<_>, EffectLawV3Error>>()?;
    canonical_bindings.sort();
    for (ordinal, (canonical_node, value_type_code)) in canonical_bindings.into_iter().enumerate() {
        relation_program.push(CanonicalRelationClauseV3 {
            relation_code: EFFECT_REL_REQUIRE,
            lhs: canonical_node,
            rhs: None,
            argument_ordinal: Some(
                u16::try_from(ordinal).map_err(|_| EffectLawV3Error::OverBudget)?,
            ),
            constant_type_code: Some(value_type_code),
            constant_sha256: None,
        });
    }
    let call_node = graph
        .nodes
        .iter()
        .find(|node| node.operation_code == Some(EFFECT_OPERATION_CALL_V3))
        .map(|node| mapped(&physical_to_canonical, node.physical_node))
        .transpose()?
        .ok_or(EffectLawV3Error::IncompleteEffectDelta)?;
    let mut canonical_constants = observation
        .constants
        .iter()
        .map(|constant| (constant.value_type_code, constant.value_sha256.clone()))
        .collect::<Vec<_>>();
    canonical_constants.sort();
    for (ordinal, (value_type_code, value_sha256)) in canonical_constants.into_iter().enumerate() {
        relation_program.push(CanonicalRelationClauseV3 {
            relation_code: EFFECT_REL_CONSTANT,
            lhs: call_node,
            rhs: None,
            argument_ordinal: Some(
                u16::try_from(ordinal).map_err(|_| EffectLawV3Error::OverBudget)?,
            ),
            constant_type_code: Some(value_type_code),
            constant_sha256: Some(value_sha256),
        });
    }
    relation_program.sort();
    relation_program.dedup();

    let normalized_atoms =
        normalized_invariant_atoms(&observation.delta, &physical_to_canonical, hypothesis)?;
    let effect_invariant_root_sha256 = evidence::sha256_serialized(&normalized_atoms)?;
    let typed_constants = relation_program
        .iter()
        .filter(|clause| clause.relation_code == EFFECT_REL_CONSTANT)
        .collect::<Vec<_>>();
    let typed_constants_root_sha256 = evidence::sha256_serialized(&typed_constants)?;
    let completion_status_renderer_root_sha256 = normalized_atom_class_root(
        &normalized_atoms,
        &[EFFECT_ATOM_POSTCONDITION, EFFECT_ATOM_RENDERER],
    )?;
    let temporal_cardinality_root_sha256 = normalized_atom_class_root(
        &normalized_atoms,
        &[EFFECT_ATOM_TEMPORAL, EFFECT_ATOM_CARDINALITY],
    )?;
    let preserved_nodes = preserved_frame_nodes(graph, &physical_to_canonical)?;
    if preserved_nodes.is_empty() {
        return Err(EffectLawV3Error::IncompleteEffectDelta);
    }
    let preserved_frame_root_sha256 = evidence::sha256_serialized(&preserved_nodes)?;
    let action_equivalence_root_sha256 = evidence::sha256_serialized(&(
        &relation_program,
        &effect_invariant_root_sha256,
        &preserved_frame_root_sha256,
    ))?;
    let law = CanonicalEffectLawV3 {
        schema: CANONICAL_EFFECT_LAW_SCHEMA_V3.to_owned(),
        ir_version: EFFECT_LAW_IR_VERSION_V3,
        dictionary_root_sha256: dictionary.root_sha256.clone(),
        quotient_hypothesis_root_sha256: hypothesis.root_sha256.clone(),
        topology_nodes,
        topology_edges,
        relation_program,
        effect_invariant_root_sha256,
        preserved_frame_root_sha256,
        action_equivalence_root_sha256,
    };
    let law_bytes = law.canonical_bytes()?;
    let mapping = physical_to_canonical
        .into_iter()
        .map(|(physical_node, canonical_node)| CanonicalNodeMappingV3 {
            physical_node,
            canonical_node,
        })
        .collect();
    Ok(CanonicalizationCandidate {
        topology_bytes,
        law_bytes,
        law,
        mapping,
        typed_constants_root_sha256,
        completion_status_renderer_root_sha256,
        temporal_cardinality_root_sha256,
    })
}

fn normalized_atom_class_root(
    atoms: &[Value],
    class_codes: &[u16],
) -> Result<String, EffectLawV3Error> {
    let selected = atoms
        .iter()
        .filter(|atom| {
            atom.get("class_code")
                .and_then(Value::as_u64)
                .and_then(|code| u16::try_from(code).ok())
                .is_some_and(|code| class_codes.contains(&code))
        })
        .collect::<Vec<_>>();
    evidence::sha256_serialized(&selected)
}

fn mapped(mapping: &BTreeMap<u16, u16>, physical: u16) -> Result<u16, EffectLawV3Error> {
    mapping
        .get(&physical)
        .copied()
        .ok_or(EffectLawV3Error::IncompleteEffectDelta)
}

fn normalized_invariant_atoms(
    delta: &EffectDeltaContractV3,
    mapping: &BTreeMap<u16, u16>,
    hypothesis: &EffectQuotientHypothesisV3,
) -> Result<Vec<Value>, EffectLawV3Error> {
    let retained = delta
        .exact_atoms
        .iter()
        .filter(|item| !hypothesis.projected_atom_classes.contains(&item.class_code))
        .collect::<Vec<_>>();
    let labels = retained
        .iter()
        .flat_map(|item| alpha_role_labels(&item.atom))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if factorial(labels.len()).is_none_or(|count| count > MAX_CANONICAL_PERMUTATIONS_V3) {
        return Err(EffectLawV3Error::OverBudget);
    }
    let mut permutation = (0..labels.len()).collect::<Vec<_>>();
    let mut best: Option<(Vec<u8>, Vec<Value>)> = None;
    enumerate_permutations(&mut permutation, 0, &mut |ordered| {
        let label_mapping = ordered
            .iter()
            .enumerate()
            .map(|(canonical, source)| {
                Ok((
                    labels[*source].clone(),
                    u16::try_from(canonical).map_err(|_| EffectLawV3Error::OverBudget)?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, EffectLawV3Error>>()?;
        let mut atoms = retained
            .iter()
            .map(|item| normalize_atom(item, mapping, &label_mapping))
            .collect::<Result<Vec<_>, EffectLawV3Error>>()?;
        atoms.sort_by_cached_key(|atom| canonical_json_bytes(atom).unwrap_or_default());
        let bytes = canonical_json_bytes(&atoms).map_err(|_| EffectLawV3Error::Serialization)?;
        if best.as_ref().is_none_or(|(current, _)| bytes < *current) {
            best = Some((bytes, atoms));
        }
        Ok(())
    })?;
    best.map(|(_, atoms)| atoms)
        .ok_or(EffectLawV3Error::IncompleteEffectDelta)
}

fn alpha_role_labels(atom: &RelationAtom) -> Vec<String> {
    match atom {
        RelationAtom::TypedEquality {
            left_role,
            right_role,
        } => vec![left_role.clone(), right_role.clone()],
        RelationAtom::Cardinality { role, .. } => vec![role.clone()],
        RelationAtom::TemporalEdge {
            predecessor,
            successor,
        } => vec![predecessor.clone(), successor.clone()],
        _ => Vec::new(),
    }
}

fn normalize_atom(
    item: &ExactEffectAtomV3,
    mapping: &BTreeMap<u16, u16>,
    label_mapping: &BTreeMap<String, u16>,
) -> Result<Value, EffectLawV3Error> {
    let atom = match &item.atom {
        RelationAtom::TypedSlot {
            slot_id,
            value_type,
            source,
            ..
        } => json!({
            "kind": "typed_slot",
            "node": mapped(mapping, *slot_id)?,
            "value_type": value_type,
            "source": source,
        }),
        RelationAtom::SlotEquality {
            left_slot,
            right_slot,
        } => {
            let left = mapped(mapping, *left_slot)?;
            let right = mapped(mapping, *right_slot)?;
            json!({"kind": "slot_equality", "left": left.min(right), "right": left.max(right)})
        }
        RelationAtom::UniqueSlot { slot_id } => {
            json!({"kind": "unique_slot", "node": mapped(mapping, *slot_id)?})
        }
        RelationAtom::ObservationSelector { slot_id, selector } => json!({
            "kind": "observation_selector",
            "node": mapped(mapping, *slot_id)?,
            "value_type": selector_value_type(selector),
        }),
        RelationAtom::ActionRoleArgument {
            slot_id,
            value_type,
            ..
        } => json!({
            "kind": "action_role_argument",
            "node": mapped(mapping, *slot_id)?,
            "value_type": value_type,
        }),
        RelationAtom::ActionIntegerArgument { value, .. } => json!({
            "kind": "action_integer_argument",
            "value": value,
        }),
        RelationAtom::ActionStringArgument { value, .. } => json!({
            "kind": "action_string_argument",
            "value": value,
        }),
        RelationAtom::ActionBooleanArgument { value, .. } => json!({
            "kind": "action_boolean_argument",
            "value": value,
        }),
        RelationAtom::TypedEquality {
            left_role,
            right_role,
        } => {
            let left = mapped_label(label_mapping, left_role)?;
            let right = mapped_label(label_mapping, right_role)?;
            json!({
                "kind": "typed_equality",
                "left_role": left.min(right),
                "right_role": left.max(right),
            })
        }
        RelationAtom::Cardinality { role, count } => json!({
            "kind": "cardinality",
            "role": mapped_label(label_mapping, role)?,
            "count": count,
        }),
        RelationAtom::TemporalEdge {
            predecessor,
            successor,
        } => json!({
            "kind": "temporal_edge",
            "predecessor": mapped_label(label_mapping, predecessor)?,
            "successor": mapped_label(label_mapping, successor)?,
        }),
        atom => serde_json::to_value(atom).map_err(|_| EffectLawV3Error::Serialization)?,
    };
    Ok(json!({
        "phase": item.phase,
        "class_code": item.class_code,
        "atom": atom,
    }))
}

fn mapped_label(mapping: &BTreeMap<String, u16>, label: &str) -> Result<u16, EffectLawV3Error> {
    mapping
        .get(label)
        .copied()
        .ok_or(EffectLawV3Error::IncompleteEffectDelta)
}

fn selector_value_type(selector: &crate::ResponseValueSelector) -> Option<crate::AtomValueType> {
    use crate::ResponseValueSelector;
    match selector {
        ResponseValueSelector::ContinuationHandle { value_type }
        | ResponseValueSelector::UniqueScalar { value_type }
        | ResponseValueSelector::UniqueTurnScalar { value_type }
        | ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | ResponseValueSelector::JsonField { value_type, .. }
        | ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | ResponseValueSelector::RequestReferencedJsonField { value_type }
        | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | ResponseValueSelector::TurnOutputLine { value_type, .. }
        | ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputLine { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarFromEnd { value_type, .. } => {
            Some(*value_type)
        }
        ResponseValueSelector::CommandOutputBody
        | ResponseValueSelector::RequestLastToken
        | ResponseValueSelector::RequestUniqueLiteral => None,
    }
}

fn preserved_frame_nodes(
    graph: &PhysicalEffectGraphV3,
    mapping: &BTreeMap<u16, u16>,
) -> Result<Vec<u16>, EffectLawV3Error> {
    let mut preserved = BTreeSet::new();
    for edge in &graph.edges {
        if !matches!(edge.relation_code, EFFECT_REL_EQUAL | EFFECT_REL_COPY) {
            continue;
        }
        for physical in [edge.from, edge.to] {
            if graph.nodes.iter().any(|node| {
                node.physical_node == physical
                    && matches!(
                        node.source,
                        EffectSource::Request | EffectSource::Observation
                    )
            }) {
                preserved.insert(mapped(mapping, physical)?);
            }
        }
    }
    Ok(preserved.into_iter().collect())
}

fn restart_bundle_digest(
    law: &CanonicalEffectLawV3,
    proofs: &[ObservationCanonicalProofV3],
    proof_set_root_sha256: &str,
) -> Result<String, EffectLawV3Error> {
    evidence::sha256_serialized(&(
        EFFECT_LAW_RESTART_BUNDLE_SCHEMA_V3,
        law,
        proofs,
        proof_set_root_sha256,
    ))
}

pub(super) fn restart_bundle_from_bytes(
    bytes: &[u8],
    trusted_evidence: &TrustedEffectEvidenceSetV3,
    expected_bundle_root: &TrustedEffectLawBundleRootV3,
) -> Result<EffectLawRestartBundleV3, EffectLawV3Error> {
    trust::validate_effect_law_bundle_root(bytes, trusted_evidence, expected_bundle_root)?;
    let wire: EffectLawRestartBundleWireV3 =
        serde_json::from_slice(bytes).map_err(|_| EffectLawV3Error::InvalidRestartBundle)?;
    if wire.schema != EFFECT_LAW_RESTART_BUNDLE_SCHEMA_V3
        || wire.law.schema != CANONICAL_EFFECT_LAW_SCHEMA_V3
        || wire.law.ir_version != EFFECT_LAW_IR_VERSION_V3
        || !valid_nonzero_sha256(&wire.law.dictionary_root_sha256)
        || !valid_nonzero_sha256(&wire.law.quotient_hypothesis_root_sha256)
        || !valid_nonzero_sha256(&wire.law.effect_invariant_root_sha256)
        || !valid_nonzero_sha256(&wire.law.preserved_frame_root_sha256)
        || !valid_nonzero_sha256(&wire.law.action_equivalence_root_sha256)
        || wire.proofs.is_empty()
    {
        return Err(EffectLawV3Error::InvalidRestartBundle);
    }
    trust::validate_effect_law_bundle_identity(&wire.law, expected_bundle_root)?;
    validate_restart_law(&wire.law)?;
    let mut proofs = wire.proofs;
    proofs.sort_by(|left, right| left.observation_sha256.cmp(&right.observation_sha256));
    if proofs
        .windows(2)
        .any(|pair| pair[0].observation_sha256 == pair[1].observation_sha256)
        || proofs.iter().any(|proof| {
            !valid_nonzero_sha256(&proof.observation_sha256)
                || !valid_nonzero_sha256(&proof.evidence_ref_sha256)
                || !valid_nonzero_sha256(&proof.transition_sha256)
                || !valid_nonzero_sha256(&proof.episode_lineage_sha256)
                || !valid_nonzero_sha256(&proof.surface_root_sha256)
                || !valid_nonzero_sha256(&proof.physical_program_id)
                || !valid_nonzero_sha256(&proof.exact_delta_root_sha256)
                || !valid_nonzero_sha256(&proof.capture_receipt_root_sha256)
                || !valid_nonzero_sha256(&proof.parity_receipt_root_sha256)
                || !valid_nonzero_sha256(&proof.verifier_root_sha256)
                || !valid_nonzero_sha256(&proof.resolver_root_sha256)
                || !valid_nonzero_sha256(&proof.trust_manifest_root_sha256)
                || !valid_nonzero_sha256(&proof.observed_state_root_sha256)
                || !valid_nonzero_sha256(&proof.verified_delta_receipt_root_sha256)
                || !valid_nonzero_sha256(&proof.delta_verifier_root_sha256)
                || !valid_restart_mapping(&proof.node_mapping, wire.law.topology_nodes.len())
        })
    {
        return Err(EffectLawV3Error::InvalidRestartBundle);
    }
    let proof_set_root_sha256 = evidence::sha256_serialized(&proofs)?;
    if proof_set_root_sha256 != wire.proof_set_root_sha256
        || restart_bundle_digest(&wire.law, &proofs, &proof_set_root_sha256)? != wire.bundle_sha256
    {
        return Err(EffectLawV3Error::InvalidRestartBundle);
    }
    trust::validate_restart_proofs(&proofs, trusted_evidence)?;
    let bundle = EffectLawRestartBundleV3 {
        schema: wire.schema,
        law: wire.law,
        proofs,
        proof_set_root_sha256,
        bundle_sha256: wire.bundle_sha256,
    };
    if bundle.canonical_bytes()? != bytes {
        return Err(EffectLawV3Error::InvalidRestartBundle);
    }
    Ok(bundle)
}

fn validate_restart_law(law: &CanonicalEffectLawV3) -> Result<(), EffectLawV3Error> {
    if law.topology_nodes.is_empty()
        || law.topology_nodes.len() > MAX_EFFECT_NODES_V3
        || law.topology_edges.len() > MAX_EFFECT_EDGES_V3
        || law.relation_program.len() > MAX_EFFECT_EDGES_V3 + MAX_EFFECT_NODES_V3 * 2
        || law
            .topology_nodes
            .iter()
            .enumerate()
            .any(|(index, node)| usize::from(node.canonical_node) != index)
        || law.topology_edges.windows(2).any(|pair| pair[0] >= pair[1])
        || law
            .relation_program
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(EffectLawV3Error::InvalidRestartBundle);
    }
    let node_count = law.topology_nodes.len();
    if law.topology_edges.iter().any(|edge| {
        usize::from(edge.from) >= node_count
            || usize::from(edge.to) >= node_count
            || edge.relation_code == 0
    }) || law.relation_program.iter().any(|clause| {
        usize::from(clause.lhs) >= node_count
            || clause.rhs.is_some_and(|rhs| usize::from(rhs) >= node_count)
            || clause.relation_code == 0
            || clause
                .constant_sha256
                .as_ref()
                .is_some_and(|digest| !valid_nonzero_sha256(digest))
            || clause.constant_sha256.is_some() && clause.constant_type_code.is_none()
    }) {
        return Err(EffectLawV3Error::InvalidRestartBundle);
    }
    let expected_action_root = evidence::sha256_serialized(&(
        &law.relation_program,
        &law.effect_invariant_root_sha256,
        &law.preserved_frame_root_sha256,
    ))?;
    if expected_action_root != law.action_equivalence_root_sha256 {
        return Err(EffectLawV3Error::InvalidRestartBundle);
    }
    Ok(())
}

fn valid_restart_mapping(mapping: &[CanonicalNodeMappingV3], node_count: usize) -> bool {
    if mapping.len() != node_count || mapping.windows(2).any(|pair| pair[0] >= pair[1]) {
        return false;
    }
    let physical = mapping
        .iter()
        .map(|item| item.physical_node)
        .collect::<BTreeSet<_>>();
    let canonical = mapping
        .iter()
        .map(|item| item.canonical_node)
        .collect::<BTreeSet<_>>();
    physical.len() == node_count
        && canonical.len() == node_count
        && canonical
            .iter()
            .copied()
            .eq((0..node_count).filter_map(|index| u16::try_from(index).ok()))
}
