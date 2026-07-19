use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::package::{relation_frame_routing_atom_ids, response_phase_atom_ids_for_prefix};
use crate::{
    RelationAtom, RelationFrame, ResponseArgument, ResponseOperation, ResponsePackage,
    ResponsePackageOrigin, ResponsePackageProof, ResponsePackageState, ResponseProgram,
    ResponseRegistry, ResponseRoutingComparison, ResponseRoutingPredicate,
    SOURCE_NEUTRAL_EXTRACTOR_VERSION, canonical_json_sha256, ground_roles,
    partition_teacher_training_families, relation_frame_phase_margin_micro,
    relation_frame_required_observable_atom_ids, response_program_external_verifier_schema,
    response_program_required_routing_atom_ids, synthesize_response_operator,
    teacher_program_signature,
};

pub const GROUNDED_RESPONSE_PACKAGE_PREFIX: &str = "raw-phase-grounded-r15-";
pub const ROUTING_REFINEMENT_VERSION: u32 = 7;

fn continuation_extractor_generation(frame: &RelationFrame) -> Option<u32> {
    frame
        .extractor_version
        .strip_prefix("response-relation-extractor.v")?
        .parse()
        .ok()
}

fn frame_is_wait(frame: &RelationFrame) -> bool {
    frame
        .atoms
        .iter()
        .any(|atom| matches!(atom, RelationAtom::ActionFunction { value } if value == "wait"))
}

fn frame_has_current_continuation_readiness(frame: &RelationFrame) -> bool {
    !frame_is_wait(frame)
        || frame.atoms.iter().any(|atom| {
            matches!(
                atom,
                RelationAtom::Cardinality { role, .. }
                    if role == "active_pending_handle_count_band"
            )
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameRepresentationPolicy {
    support_requires_readiness: bool,
    support_generation: Option<u32>,
}

impl FrameRepresentationPolicy {
    #[must_use]
    pub fn from_support(support: &[RelationFrame]) -> Self {
        Self {
            support_requires_readiness: support
                .iter()
                .any(|row| frame_is_wait(row) && frame_has_current_continuation_readiness(row)),
            support_generation: support
                .iter()
                .filter(|row| frame_is_wait(row))
                .filter_map(continuation_extractor_generation)
                .max(),
        }
    }

    #[must_use]
    pub fn matches(self, frame: &RelationFrame) -> bool {
        (!self.support_requires_readiness || frame_has_current_continuation_readiness(frame))
            && self.support_generation.is_none_or(|generation| {
                !frame_is_wait(frame)
                    || continuation_extractor_generation(frame) == Some(generation)
            })
    }
}

#[must_use]
pub fn frame_representation_matches_support(
    support: &[RelationFrame],
    frame: &RelationFrame,
) -> bool {
    FrameRepresentationPolicy::from_support(support).matches(frame)
}

#[must_use]
pub fn frame_matches_program_action_contract(
    program: &ResponseProgram,
    frame: &RelationFrame,
) -> bool {
    if matches!(
        program.operation,
        ResponseOperation::CustomToolCallFromRoles { .. }
            | ResponseOperation::ProjectSelectedValue { .. }
            | ResponseOperation::ProjectStatus { .. }
    ) {
        return crate::synthesis::program_is_consistent(program, frame);
    }
    let hypotheses = ground_roles(frame);
    frame_matches_program_action_contract_with_grounding(
        program,
        frame,
        hypotheses.len() == 1 && hypotheses[0].competing_binding_count == 0,
    )
}

#[must_use]
pub fn frame_matches_program_action_contract_with_grounding(
    program: &ResponseProgram,
    frame: &RelationFrame,
    uniquely_grounded: bool,
) -> bool {
    if !uniquely_grounded {
        return false;
    }
    if matches!(
        program.operation,
        ResponseOperation::ProjectSelectedValue { .. } | ResponseOperation::ProjectStatus { .. }
    ) {
        return crate::synthesis::program_is_consistent(program, frame);
    }
    let (expected_action, expected_shape, arguments) = match &program.operation {
        ResponseOperation::FunctionCallFromRoles {
            function_name,
            arguments,
            ..
        } => (
            function_name.as_str(),
            "function_call",
            arguments.as_slice(),
        ),
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name,
            arguments,
            ..
        } => (
            custom_tool_name.as_str(),
            "custom_tool_call",
            arguments.as_slice(),
        ),
        _ => return false,
    };
    let action_matches = frame.atoms.iter().any(|atom| match atom {
        RelationAtom::ActionFunction { value } | RelationAtom::ActionCustomTool { value } => {
            value == expected_action
        }
        _ => false,
    });
    let shape_matches = frame.atoms.iter().any(
        |atom| matches!(atom, RelationAtom::ResponseShape { value } if value == expected_shape),
    );
    let completion_state = frame.atoms.iter().find_map(|atom| match atom {
        RelationAtom::CompletionState { value } => Some(value.as_str()),
        _ => None,
    });
    let requires_continuation = arguments.iter().any(|argument| {
        matches!(
            argument,
            ResponseArgument::Role {
                role: crate::SemanticRole::ContinuationHandle,
                ..
            }
        )
    });
    let requires_source = arguments.iter().any(|argument| {
        matches!(
            argument,
            ResponseArgument::Role {
                role: crate::SemanticRole::SourceValue,
                ..
            }
        )
    });
    let completion_matches = match (requires_continuation, requires_source) {
        (true, false) => completion_state == Some("pending"),
        (false, true) => completion_state == Some("completed"),
        _ => false,
    };
    let has_binding = frame
        .atoms
        .iter()
        .any(|atom| matches!(atom, RelationAtom::SlotEquality { .. }));
    let role_names = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ActionRoleArgument { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let integer_arguments = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ActionIntegerArgument { name, value }
                if !crate::teacher_join::is_execution_budget_argument(name) =>
            {
                Some((name.as_str(), *value))
            }
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let string_arguments = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ActionStringArgument { name, value } => {
                Some((name.as_str(), value.as_str()))
            }
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let boolean_arguments = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ActionBooleanArgument { name, value } => Some((name.as_str(), *value)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let argument_names_match = arguments.iter().all(|argument| match argument {
        ResponseArgument::Role { name, .. } => role_names.contains(name.as_str()),
        ResponseArgument::Integer { name, .. }
            if crate::teacher_join::is_execution_budget_argument(name) =>
        {
            true
        }
        ResponseArgument::Integer { name, value } => {
            integer_arguments.get(name.as_str()) == Some(value)
        }
        ResponseArgument::String { name, value } => {
            string_arguments.get(name.as_str()) == Some(&value.as_str())
        }
        ResponseArgument::Boolean { name, value } => {
            boolean_arguments.get(name.as_str()) == Some(value)
        }
    });
    action_matches && shape_matches && completion_matches && has_binding && argument_names_match
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponseProgramHint {
    pub op: String,
    pub prefix: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponseRelationObservation {
    pub schema: String,
    pub relation_id: String,
    pub observed_at: String,
    pub relation: String,
    pub program_hint: ResponseProgramHint,
    pub source_session_id_sha256: String,
    pub source_turn_id_sha256: String,
    pub surface_id_sha256: String,
    #[serde(default = "default_true")]
    pub verifier_ok: bool,
    #[serde(default)]
    pub guard_schema: String,
}

const fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponseShadowObservation {
    pub schema: String,
    pub package_id: String,
    pub observed_at: String,
    pub source_session_id_sha256: String,
    pub surface_id_sha256: String,
    pub matched_guard: bool,
    pub verifier_ok: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponseSupportManifest {
    pub schema: String,
    pub package_id: String,
    #[serde(default)]
    pub lineage_id: String,
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub routing_refinement_version: u32,
    #[serde(default)]
    pub supersedes_package_id: Option<String>,
    pub created_at_unix_nanos: u64,
    pub support_boundary_unix_nanos: u64,
    pub support_frame_ids: Vec<String>,
    pub support_session_ids: Vec<String>,
    pub support_intent_ids: Vec<String>,
    #[serde(default)]
    pub reserved_future_session_ids: Vec<String>,
    pub learned_center_atom_ids: Vec<u64>,
    pub learned_anti_center_atom_ids: Vec<u64>,
    #[serde(default)]
    pub selected_routing_atom_ids: Vec<u64>,
    #[serde(default)]
    pub selected_routing_predicates: Vec<ResponseRoutingPredicate>,
    #[serde(default)]
    pub split_negative_frame_ids: Vec<String>,
    #[serde(default)]
    pub holdout_negative_frame_ids: Vec<String>,
    #[serde(default)]
    pub split_parent_support_rows: usize,
    pub manifest_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponseSupportManifestSet {
    pub schema: String,
    pub manifests: Vec<ResponseSupportManifest>,
}

pub fn response_support_manifest_digest(
    manifest: &ResponseSupportManifest,
) -> Result<String, &'static str> {
    let mut material =
        serde_json::to_value(manifest).map_err(|_| "support_manifest_digest_serialize_failed")?;
    let object = material
        .as_object_mut()
        .ok_or("support_manifest_digest_not_object")?;
    object.remove("manifest_sha256");
    canonical_json_sha256(&material)
}

#[derive(Clone, Debug, Default)]
pub struct ResponseSupportFreezePolicy {
    pub forced_support_session_ids_by_lineage: BTreeMap<String, BTreeSet<String>>,
    pub forced_family_id_by_lineage: BTreeMap<String, u64>,
    pub generation_by_lineage: BTreeMap<String, u64>,
    pub supersedes_package_id_by_lineage: BTreeMap<String, String>,
    pub only_lineages: BTreeSet<String>,
}

pub fn freeze_source_neutral_support(
    frames: &[RelationFrame],
    created_at_unix_nanos: u64,
    wave_causal_pass: bool,
) -> ResponseSupportManifestSet {
    freeze_source_neutral_support_with_policy(
        frames,
        created_at_unix_nanos,
        wave_causal_pass,
        &ResponseSupportFreezePolicy::default(),
    )
}

pub fn freeze_source_neutral_support_with_policy(
    frames: &[RelationFrame],
    created_at_unix_nanos: u64,
    wave_causal_pass: bool,
    policy: &ResponseSupportFreezePolicy,
) -> ResponseSupportManifestSet {
    let current_continuation_generation = frames
        .iter()
        .filter(|frame| frame_is_wait(frame) && frame_has_current_continuation_readiness(frame))
        .filter_map(continuation_extractor_generation)
        .max();
    let mut eligible_frames = Vec::new();
    for frame in frames {
        // Continuation applicability depends on the number of independently
        // live handles. Frames captured before that pre-action feature existed
        // cannot prove a current continuation package.
        if let Some(generation) = current_continuation_generation
            && frame_is_wait(frame)
            && (!frame_has_current_continuation_readiness(frame)
                || continuation_extractor_generation(frame) != Some(generation))
        {
            continue;
        }
        eligible_frames.push(frame.clone());
    }
    let families = partition_teacher_training_families(&eligible_frames);
    let mut base_lineage_counts = BTreeMap::<String, usize>::new();
    for family in families.values() {
        if let Some(package) = compile_source_neutral_quarantine_packages(family, wave_causal_pass)
            .into_iter()
            .next()
        {
            let base =
                response_package_lineage_id(&package.program, &package.required_routing_atom_ids);
            *base_lineage_counts.entry(base).or_default() += 1;
        }
    }
    let manifests = families
        .into_iter()
        .filter_map(|((teacher_pool_id, teacher_signature), family)| {
            let positive_rows = family
                .iter()
                .filter(|frame| frame.verifier_label == Some(true))
                .count();
            if positive_rows < 32 {
                return None;
            }
            let provisional_package =
                compile_source_neutral_quarantine_packages(&family, wave_causal_pass)
                    .into_iter()
                    .next()?;
            let base_lineage = response_package_lineage_id(
                &provisional_package.program,
                &provisional_package.required_routing_atom_ids,
            );
            let family_lineage_is_shared =
                base_lineage_counts.get(&base_lineage).copied().unwrap_or(0) > 1;
            let inferred_lineage_id = if family_lineage_is_shared {
                family_lineage_id(
                    teacher_pool_id,
                    &provisional_package.program,
                    &provisional_package.required_routing_atom_ids,
                )
            } else {
                base_lineage
            };
            let lineage_id = policy
                .forced_family_id_by_lineage
                .iter()
                .find_map(|(lineage, forced_family_id)| {
                    (*forced_family_id == teacher_pool_id).then_some(lineage.clone())
                })
                .unwrap_or(inferred_lineage_id);
            if !policy.only_lineages.is_empty() && !policy.only_lineages.contains(&lineage_id) {
                return None;
            }
            let forced_support_sessions = policy
                .forced_support_session_ids_by_lineage
                .get(&lineage_id)
                .cloned()
                .unwrap_or_default();
            let mut session_last_seen = BTreeMap::new();
            for frame in &family {
                session_last_seen
                    .entry(frame.session_id_sha256.clone())
                    .and_modify(|seen: &mut u64| *seen = (*seen).max(frame.observed_at_unix_nanos))
                    .or_insert(frame.observed_at_unix_nanos);
            }
            let mut ordered_sessions = session_last_seen.into_iter().collect::<Vec<_>>();
            ordered_sessions.sort_by_key(|(session, seen)| (*seen, session.clone()));
            let session_rows = family
                .iter()
                .filter(|frame| frame.verifier_label == Some(true))
                .fold(BTreeMap::<String, usize>::new(), |mut rows, frame| {
                    *rows.entry(frame.session_id_sha256.clone()).or_default() += 1;
                    rows
                });
            let mut reserved_future_sessions = BTreeSet::new();
            let mut remaining_support_rows = positive_rows;
            for (session, _) in ordered_sessions.iter().rev() {
                if reserved_future_sessions.len() == 3 {
                    break;
                }
                if forced_support_sessions.contains(session) {
                    continue;
                }
                let rows = session_rows.get(session).copied().unwrap_or(0);
                if rows == 0 {
                    continue;
                }
                if remaining_support_rows.saturating_sub(rows) < 32 {
                    continue;
                }
                reserved_future_sessions.insert(session.clone());
                remaining_support_rows = remaining_support_rows.saturating_sub(rows);
            }
            let mut selected_sessions = ordered_sessions
                .iter()
                .map(|(session, _)| session.clone())
                .filter(|session| !reserved_future_sessions.contains(session))
                .collect::<BTreeSet<_>>();
            selected_sessions.extend(forced_support_sessions);
            let mut support = family
                .iter()
                .filter(|frame| selected_sessions.contains(&frame.session_id_sha256))
                .cloned()
                .collect::<Vec<_>>();
            let base_package =
                compile_source_neutral_quarantine_packages(&support, wave_causal_pass)
                    .into_iter()
                    .next()?;
            let verifier_negatives = frames
                .iter()
                .filter(|frame| {
                    frame_representation_matches_support(&support, frame)
                        && frame.verifier_label == Some(false)
                        && selected_sessions.contains(&frame.session_id_sha256)
                        && !frame_matches_program_action_contract(&base_package.program, frame)
                })
                .cloned()
                .collect::<Vec<_>>();
            let cross_family_negatives = frames
                .iter()
                .filter(|frame| {
                    frame_representation_matches_support(&support, frame)
                        && frame.verifier_label == Some(true)
                        && selected_sessions.contains(&frame.session_id_sha256)
                        && teacher_program_signature(frame)
                            .is_some_and(|signature| signature != teacher_signature)
                })
                .cloned()
                .collect::<Vec<_>>();
            let equivalent_action_event_ids = frames
                .iter()
                .filter(|frame| {
                    selected_sessions.contains(&frame.session_id_sha256)
                        && frame_matches_program_action_contract(&base_package.program, frame)
                })
                .map(|frame| frame.event_id_sha256.as_str())
                .collect::<BTreeSet<_>>();
            let mut training_negatives = verifier_negatives;
            training_negatives.retain(|frame| {
                !equivalent_action_event_ids.contains(frame.event_id_sha256.as_str())
            });
            training_negatives.extend(cross_family_negatives);
            let holdout_verifier_negatives = frames
                .iter()
                .filter(|frame| {
                    frame_representation_matches_support(&support, frame)
                        && reserved_future_sessions.contains(&frame.session_id_sha256)
                        && ((frame.verifier_label == Some(false)
                            && !frame_matches_program_action_contract(
                                &base_package.program,
                                frame,
                            ))
                            || (frame.verifier_label == Some(true)
                                && teacher_program_signature(frame)
                                    .is_some_and(|signature| signature != teacher_signature)))
                })
                .filter(|frame| {
                    !frames.iter().any(|sibling| {
                        sibling.event_id_sha256 == frame.event_id_sha256
                            && frame_matches_program_action_contract(&base_package.program, sibling)
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            let split_parent_support_rows = support
                .iter()
                .filter(|frame| frame.verifier_label == Some(true))
                .count();
            let routing_refinement = select_clean_routing_refinement(
                &support,
                &training_negatives,
                &base_package.phase_centers,
            );
            let selected_routing_atom_ids = routing_refinement.exact_atom_ids.clone();
            let selected_routing_predicates = routing_refinement.predicates.clone();
            if !routing_refinement.is_empty() {
                support.retain(|frame| routing_refinement.matches_frame(frame));
            }
            if support
                .iter()
                .filter(|frame| frame.verifier_label == Some(true))
                .count()
                < 32
            {
                return None;
            }
            let mut package =
                compile_source_neutral_quarantine_packages(&support, wave_causal_pass)
                    .into_iter()
                    .next()?;
            if !routing_refinement.is_empty() {
                package.phase_centers = base_package.phase_centers.clone();
                package
                    .required_routing_atom_ids
                    .extend(selected_routing_atom_ids.iter().copied());
                package.required_routing_atom_ids.sort_unstable();
                package.required_routing_atom_ids.dedup();
                package
                    .phase_centers
                    .extend(selected_routing_atom_ids.iter().copied());
                package.phase_centers.extend(
                    selected_routing_predicates
                        .iter()
                        .map(ResponseRoutingPredicate::phase_atom_id),
                );
                package.phase_centers.sort_unstable();
                package.phase_centers.dedup();
                package.routing_predicates = selected_routing_predicates.clone();
            }
            let base_lineage =
                response_package_lineage_id(&package.program, &package.required_routing_atom_ids);
            let lineage_id = if family_lineage_is_shared {
                family_lineage_id(
                    teacher_pool_id,
                    &package.program,
                    &package.required_routing_atom_ids,
                )
            } else {
                base_lineage
            };
            let generation = policy
                .generation_by_lineage
                .get(&lineage_id)
                .copied()
                .unwrap_or(1);
            package.package_id = grounded_response_package_id(&lineage_id, generation);
            let mut support_frame_ids = support
                .iter()
                .filter(|frame| frame.verifier_label == Some(true))
                .map(|frame| frame.frame_id_sha256.clone())
                .collect::<Vec<_>>();
            let mut support_session_ids = selected_sessions.iter().cloned().collect::<Vec<_>>();
            let mut support_intent_ids = support
                .iter()
                .filter(|frame| frame.verifier_label == Some(true))
                .map(|frame| frame.client_intent_id_sha256.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            support_frame_ids.sort();
            support_frame_ids.dedup();
            support_session_ids.sort();
            support_intent_ids.sort();
            let reserved_future_session_ids = reserved_future_sessions.into_iter().collect();
            let support_boundary_unix_nanos = support
                .iter()
                .map(|frame| frame.observed_at_unix_nanos)
                .chain(
                    training_negatives
                        .iter()
                        .map(|frame| frame.observed_at_unix_nanos),
                )
                .max()
                .unwrap_or(0);
            let mut manifest = ResponseSupportManifest {
                schema: "nando.response-support-manifest.v1".to_owned(),
                package_id: package.package_id,
                lineage_id: lineage_id.clone(),
                generation,
                routing_refinement_version: ROUTING_REFINEMENT_VERSION,
                supersedes_package_id: policy
                    .supersedes_package_id_by_lineage
                    .get(&lineage_id)
                    .cloned(),
                created_at_unix_nanos,
                support_boundary_unix_nanos,
                support_frame_ids,
                support_session_ids,
                support_intent_ids,
                reserved_future_session_ids,
                learned_center_atom_ids: package.phase_centers,
                learned_anti_center_atom_ids: package.anti_centers,
                selected_routing_atom_ids,
                selected_routing_predicates,
                split_negative_frame_ids: training_negatives
                    .iter()
                    .map(|frame| frame.frame_id_sha256.clone())
                    .collect(),
                holdout_negative_frame_ids: holdout_verifier_negatives
                    .iter()
                    .map(|frame| frame.frame_id_sha256.clone())
                    .collect(),
                split_parent_support_rows,
                manifest_sha256: String::new(),
            };
            manifest.manifest_sha256 = response_support_manifest_digest(&manifest).ok()?;
            Some(manifest)
        })
        .collect();
    ResponseSupportManifestSet {
        schema: "nando.response-support-manifest-set.v1".to_owned(),
        manifests,
    }
}

fn select_clean_routing_refinement(
    support: &[RelationFrame],
    negatives: &[RelationFrame],
    base_center: &[u64],
) -> RoutingRefinement {
    let continuation_family = support.iter().any(frame_is_wait);
    let base = base_center.iter().copied().collect::<BTreeSet<_>>();
    // Exact cardinality atoms describe a particular conversation age, not a
    // stable applicability relation. Cardinalities may participate only via
    // the range/set predicates synthesized below, where negative evidence can
    // prove the boundary explicitly.
    let volatile_cardinality_atoms = support
        .iter()
        .chain(negatives.iter())
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::Cardinality { role, count } => Some(crate::package::stable_atom_id(
                &format!("cardinality:{role}:{count}"),
            )),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let support_atoms = support
        .iter()
        .map(|frame| {
            relation_frame_routing_atom_ids(frame)
                .into_iter()
                .filter(|atom| !base.contains(atom) && !volatile_cardinality_atoms.contains(atom))
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>();
    let negative_atoms = negatives
        .iter()
        .map(|frame| {
            relation_frame_routing_atom_ids(frame)
                .into_iter()
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>();
    let mut universe = support_atoms
        .iter()
        .flat_map(|atoms| atoms.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    universe.retain(|atom| {
        support_atoms
            .iter()
            .filter(|atoms| atoms.contains(atom))
            .count()
            >= 32
    });
    universe.sort_by(|left, right| {
        let left_negatives = negative_atoms
            .iter()
            .filter(|atoms| atoms.contains(left))
            .count();
        let right_negatives = negative_atoms
            .iter()
            .filter(|atoms| atoms.contains(right))
            .count();
        let left_support = support_atoms
            .iter()
            .filter(|atoms| atoms.contains(left))
            .count();
        let right_support = support_atoms
            .iter()
            .filter(|atoms| atoms.contains(right))
            .count();
        left_negatives
            .cmp(&right_negatives)
            .then_with(|| right_support.cmp(&left_support))
            .then_with(|| left.cmp(right))
    });
    universe.truncate(32);
    universe.sort_unstable();
    let mut exact_candidates = universe
        .iter()
        .copied()
        .map(|atom| vec![atom])
        .collect::<Vec<_>>();
    for (left_index, left) in universe.iter().enumerate() {
        for right in universe.iter().skip(left_index + 1) {
            exact_candidates.push(vec![*left, *right]);
        }
    }
    let mut beam = vec![Vec::<u64>::new()];
    for _depth in 0..4 {
        let mut expanded = Vec::<(Vec<u64>, usize, usize)>::new();
        for prefix in &beam {
            let minimum_atom = prefix.last().copied();
            for atom in universe
                .iter()
                .copied()
                .filter(|atom| minimum_atom.is_none_or(|minimum| *atom > minimum))
            {
                let mut candidate = prefix.clone();
                candidate.push(atom);
                let retained = support_atoms
                    .iter()
                    .filter(|atoms| candidate.iter().all(|atom| atoms.contains(atom)))
                    .count();
                if retained < 32 {
                    continue;
                }
                let surviving_negatives = negative_atoms
                    .iter()
                    .filter(|atoms| candidate.iter().all(|atom| atoms.contains(atom)))
                    .count();
                if surviving_negatives == 0 {
                    exact_candidates.push(candidate.clone());
                }
                expanded.push((candidate, surviving_negatives, retained));
            }
        }
        expanded.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| left.0.cmp(&right.0))
        });
        expanded.dedup_by(|left, right| left.0 == right.0);
        beam = expanded
            .into_iter()
            .take(64)
            .map(|(candidate, _, _)| candidate)
            .collect();
        if beam.is_empty() {
            break;
        }
    }
    let mut candidates = Vec::new();
    for candidate in exact_candidates {
        if negative_atoms
            .iter()
            .any(|atoms| candidate.iter().all(|atom| atoms.contains(atom)))
        {
            continue;
        }
        let retained = support_atoms
            .iter()
            .filter(|atoms| candidate.iter().all(|atom| atoms.contains(atom)))
            .count();
        if retained < 32 {
            continue;
        }
        candidates.push(RoutingRefinement {
            exact_atom_ids: candidate,
            predicates: Vec::new(),
            retained_support_rows: retained,
        });
    }

    let cardinalities = support
        .iter()
        .chain(negatives.iter())
        .flat_map(|frame| frame.atoms.iter())
        .filter_map(|atom| match atom {
            RelationAtom::Cardinality { role, count }
                if !continuation_family
                    || matches!(
                        role.as_str(),
                        "active_pending_handle_count_band" | "turn_call_shape_count_band"
                    ) =>
            {
                Some((role.clone(), *count))
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut predicate_primitives = Vec::new();
    for (role, threshold) in &cardinalities {
        for comparison in [
            ResponseRoutingComparison::AtMost,
            ResponseRoutingComparison::AtLeast,
        ] {
            let predicate = ResponseRoutingPredicate {
                role: role.clone(),
                comparison,
                threshold: *threshold,
                allowed_counts: Vec::new(),
            };
            let surviving_negatives = negatives
                .iter()
                .filter(|frame| routing_predicate_matches_frame(&predicate, frame))
                .count();
            let retained = support
                .iter()
                .filter(|frame| routing_predicate_matches_frame(&predicate, frame))
                .count();
            if retained >= 32 && surviving_negatives < negatives.len() {
                predicate_primitives.push(predicate.clone());
            }
            if retained >= 32 && surviving_negatives == 0 {
                candidates.push(RoutingRefinement {
                    exact_atom_ids: Vec::new(),
                    predicates: vec![predicate],
                    retained_support_rows: retained,
                });
            }
        }
    }
    let cardinality_roles = cardinalities
        .iter()
        .map(|(role, _)| role.clone())
        .collect::<BTreeSet<_>>();
    for role in cardinality_roles {
        let negative_counts = negatives
            .iter()
            .flat_map(|frame| frame.atoms.iter())
            .filter_map(|atom| match atom {
                RelationAtom::Cardinality {
                    role: atom_role,
                    count,
                } if atom_role == &role => Some(*count),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let allowed_counts = support
            .iter()
            .flat_map(|frame| frame.atoms.iter())
            .filter_map(|atom| match atom {
                RelationAtom::Cardinality {
                    role: atom_role,
                    count,
                } if atom_role == &role && !negative_counts.contains(count) => Some(*count),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let predicate = ResponseRoutingPredicate {
            role,
            comparison: ResponseRoutingComparison::OneOf,
            threshold: 0,
            allowed_counts,
        };
        let retained = support
            .iter()
            .filter(|frame| routing_predicate_matches_frame(&predicate, frame))
            .count();
        if retained >= 32 && !predicate.allowed_counts.is_empty() {
            predicate_primitives.push(predicate.clone());
            candidates.push(RoutingRefinement {
                exact_atom_ids: Vec::new(),
                predicates: vec![predicate],
                retained_support_rows: retained,
            });
        }
    }

    predicate_primitives.sort_by_key(|predicate| predicate.phase_atom_id());
    predicate_primitives.dedup();
    for (left_index, left) in predicate_primitives.iter().enumerate() {
        for right in predicate_primitives.iter().skip(left_index + 1) {
            if left == right {
                continue;
            }
            let refinement = RoutingRefinement {
                exact_atom_ids: Vec::new(),
                predicates: vec![left.clone(), right.clone()],
                retained_support_rows: 0,
            };
            if negatives
                .iter()
                .any(|frame| refinement.matches_frame(frame))
            {
                continue;
            }
            let retained = support
                .iter()
                .filter(|frame| refinement.matches_frame(frame))
                .count();
            if retained >= 32 {
                candidates.push(RoutingRefinement {
                    retained_support_rows: retained,
                    ..refinement
                });
            }
        }
    }

    candidates
        .into_iter()
        .max_by(|left, right| {
            left.retained_support_rows
                .cmp(&right.retained_support_rows)
                .then_with(|| right.complexity().cmp(&left.complexity()))
                .then_with(|| right.stable_key().cmp(&left.stable_key()))
        })
        .unwrap_or_default()
}

pub(crate) fn apply_clean_routing_refinement(
    package: &mut ResponsePackage,
    support: &[RelationFrame],
    negatives: &[RelationFrame],
) {
    let refinement = select_clean_routing_refinement(support, negatives, &package.phase_centers);
    if refinement.is_empty() {
        return;
    }
    package
        .required_routing_atom_ids
        .extend(refinement.exact_atom_ids.iter().copied());
    package.required_routing_atom_ids.sort_unstable();
    package.required_routing_atom_ids.dedup();
    package
        .phase_centers
        .extend(refinement.exact_atom_ids.iter().copied());
    package.phase_centers.extend(
        refinement
            .predicates
            .iter()
            .map(ResponseRoutingPredicate::phase_atom_id),
    );
    package.phase_centers.sort_unstable();
    package.phase_centers.dedup();
    package.routing_predicates = refinement.predicates;
}

#[derive(Clone, Debug, Default)]
struct RoutingRefinement {
    exact_atom_ids: Vec<u64>,
    predicates: Vec<ResponseRoutingPredicate>,
    retained_support_rows: usize,
}

impl RoutingRefinement {
    fn is_empty(&self) -> bool {
        self.exact_atom_ids.is_empty() && self.predicates.is_empty()
    }

    fn complexity(&self) -> usize {
        self.exact_atom_ids.len()
            + self
                .predicates
                .iter()
                .map(|predicate| 1 + predicate.allowed_counts.len())
                .sum::<usize>()
    }

    fn stable_key(&self) -> String {
        serde_json::to_string(&(&self.exact_atom_ids, &self.predicates)).unwrap_or_default()
    }

    fn matches_frame(&self, frame: &RelationFrame) -> bool {
        let atoms = relation_frame_routing_atom_ids(frame)
            .into_iter()
            .collect::<BTreeSet<_>>();
        self.exact_atom_ids.iter().all(|atom| atoms.contains(atom))
            && self
                .predicates
                .iter()
                .all(|predicate| routing_predicate_matches_frame(predicate, frame))
    }
}

fn routing_predicate_matches_frame(
    predicate: &ResponseRoutingPredicate,
    frame: &RelationFrame,
) -> bool {
    frame.atoms.iter().any(|atom| {
        matches!(
            atom,
            RelationAtom::Cardinality { role, count }
                if role == &predicate.role && predicate.matches_count(*count)
        )
    })
}

pub fn compile_source_neutral_quarantine_packages(
    frames: &[RelationFrame],
    wave_causal_pass: bool,
) -> Vec<ResponsePackage> {
    let families = partition_teacher_training_families(frames);
    let mut base_lineage_counts = BTreeMap::<String, usize>::new();
    for family in families.values() {
        if let Ok(operator) = synthesize_response_operator(family) {
            let required = response_program_required_routing_atom_ids(&operator.candidate.program);
            let base = response_package_lineage_id(&operator.candidate.program, &required);
            *base_lineage_counts.entry(base).or_default() += 1;
        }
    }
    families
        .into_iter()
        .filter_map(|((family_id, _teacher_signature), family)| {
            let operator = synthesize_response_operator(&family).ok()?;
            let positive_family = family
                .iter()
                .filter(|frame| frame.verifier_label == Some(true))
                .collect::<Vec<_>>();
            if positive_family.is_empty() {
                return None;
            }
            let sessions = positive_family
                .iter()
                .map(|frame| frame.session_id_sha256.as_str())
                .collect::<BTreeSet<_>>();
            let surfaces = positive_family
                .iter()
                .flat_map(|frame| frame.atoms.iter())
                .filter_map(|atom| match atom {
                    RelationAtom::ToolKind { value } => Some(value.as_str()),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            let mut positive_counts = BTreeMap::new();
            for frame in &positive_family {
                for atom_id in relation_frame_routing_atom_ids(frame) {
                    *positive_counts.entry(atom_id).or_insert(0_usize) += 1;
                }
            }
            let learned_center = positive_counts
                .into_iter()
                .filter_map(|(atom_id, count)| (count == positive_family.len()).then_some(atom_id))
                .collect::<Vec<_>>();
            let mut required_routing_atom_ids =
                response_program_required_routing_atom_ids(&operator.candidate.program);
            if let Some(first) = positive_family.first() {
                let mut common = relation_frame_required_observable_atom_ids(first);
                common.retain(|atom| {
                    positive_family.iter().skip(1).all(|frame| {
                        relation_frame_required_observable_atom_ids(frame)
                            .binary_search(atom)
                            .is_ok()
                    })
                });
                required_routing_atom_ids.extend(common);
            }
            required_routing_atom_ids.sort_unstable();
            required_routing_atom_ids.dedup();
            if !positive_family
                .iter()
                .all(|frame| !relation_frame_required_observable_atom_ids(frame).is_empty())
            {
                return None;
            }
            let base_lineage = response_package_lineage_id(
                &operator.candidate.program,
                &required_routing_atom_ids,
            );
            let lineage_id = if base_lineage_counts.get(&base_lineage).copied().unwrap_or(0) > 1 {
                family_lineage_id(
                    family_id,
                    &operator.candidate.program,
                    &required_routing_atom_ids,
                )
            } else {
                base_lineage
            };
            let package_id = grounded_response_package_id(&lineage_id, 0);
            let verifier_schema =
                response_program_external_verifier_schema(&operator.candidate.program)
                    .unwrap_or("source_neutral_structure_only.v1")
                    .to_owned();
            let mut package = ResponsePackage {
                schema: "nando.response-package.v1".to_owned(),
                package_id,
                origin: ResponsePackageOrigin::GroundedSynthesis,
                state: ResponsePackageState::Quarantine,
                program: operator.candidate.program,
                verifier: Some(operator.verifier),
                routing_predicates: Vec::new(),
                required_routing_atom_ids,
                phase_centers: learned_center,
                anti_centers: Vec::new(),
                wave_margin_micro: 850_000,
                learned_wave_route: None,
                crystallized_operator: None,
                proof: ResponsePackageProof {
                    support_rows: positive_family.len(),
                    future_rows: 0,
                    distinct_sessions: sessions.len(),
                    distinct_surfaces: surfaces.len(),
                    wrong_accepts: 0,
                    runtime_parity_failures: 0,
                    exact_cache_overlap: 0,
                    wave_causal_pass,
                    verifier_schema,
                },
            };
            let negatives = family
                .iter()
                .filter(|frame| frame.verifier_label == Some(false))
                .collect::<Vec<_>>();
            if let Some(threshold) =
                calibrated_wave_margin_micro(&package, &positive_family, &negatives)
            {
                package.wave_margin_micro = threshold;
            }
            Some(package)
        })
        .collect()
}

fn calibrated_wave_margin_micro(
    package: &ResponsePackage,
    positives: &[&RelationFrame],
    negatives: &[&RelationFrame],
) -> Option<i64> {
    let mut positive_margins = positives
        .iter()
        .filter_map(|frame| relation_frame_phase_margin_micro(package, frame))
        .collect::<Vec<_>>();
    if positive_margins.len() != positives.len() || positive_margins.is_empty() {
        return None;
    }
    positive_margins.sort_unstable();
    let p10_index = positive_margins.len().saturating_sub(1) / 10;
    let threshold = positive_margins[p10_index]
        .min(package.wave_margin_micro)
        .max(1);
    let max_negative = negatives
        .iter()
        .filter_map(|frame| relation_frame_phase_margin_micro(package, frame))
        .max();
    if max_negative.is_some_and(|margin| margin >= threshold) {
        return None;
    }
    Some(threshold)
}

#[must_use]
pub fn response_package_lineage_id(
    program: &ResponseProgram,
    required_routing_atom_ids: &[u64],
) -> String {
    let argument_shape = |arguments: &[ResponseArgument]| {
        arguments
            .iter()
            .map(|argument| match argument {
                ResponseArgument::Role {
                    name,
                    role,
                    value_type,
                } => {
                    serde_json::json!({"source":"role","name":name,"role":role,"value_type":value_type})
                }
                ResponseArgument::Integer { name, .. } => {
                    serde_json::json!({"source":"integer","name":name})
                }
                ResponseArgument::String { name, .. } => {
                    serde_json::json!({"source":"string","name":name})
                }
                ResponseArgument::Boolean { name, .. } => {
                    serde_json::json!({"source":"boolean","name":name})
                }
            })
            .collect::<Vec<_>>()
    };
    let operation = match &program.operation {
        ResponseOperation::FunctionCallFromRoles {
            function_name,
            selector,
            arguments,
        } => serde_json::json!({
            "op":"function_call_from_roles",
            "function_name":function_name,
            "selector":selector,
            "arguments":argument_shape(arguments),
        }),
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
        } => serde_json::json!({
            "op":"custom_tool_call_from_roles",
            "custom_tool_name":custom_tool_name,
            "inner_tool_name":inner_tool_name,
            "selector":selector,
            "arguments":argument_shape(arguments),
            "projection":projection,
        }),
        ResponseOperation::ProjectSelectedValue { selector, .. } => serde_json::json!({
            "selector":selector,
        }),
        _ => serde_json::to_value(&program.operation).unwrap_or_default(),
    };
    let material = serde_json::to_vec(&(
        SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        operation,
        required_routing_atom_ids,
    ))
    .unwrap_or_default();
    format!("{:x}", Sha256::digest(material))
}

#[must_use]
pub fn grounded_response_package_id(lineage_id: &str, generation: u64) -> String {
    format!(
        "{GROUNDED_RESPONSE_PACKAGE_PREFIX}{}-g{generation:04}",
        &lineage_id[..16]
    )
}

fn family_lineage_id(
    family_id: u64,
    program: &ResponseProgram,
    required_routing_atom_ids: &[u64],
) -> String {
    let base = response_package_lineage_id(program, required_routing_atom_ids);
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&("family_lineage_v1", family_id, base)).unwrap_or_default(),
        )
    )
}

pub fn compile_response_registry(
    revision: u64,
    relations: &[ResponseRelationObservation],
    shadows: &[ResponseShadowObservation],
    wave_causal_pass: bool,
) -> ResponseRegistry {
    let valid = relations.iter().filter(|row| {
        row.schema == "nando.response-relation-observation.v1"
            && row.relation == "outcome_equals_request_suffix"
            && row.program_hint.op == "copy_after_prefix"
            && !row.program_hint.prefix.is_empty()
    });
    let mut prefixes = BTreeSet::new();
    let mut sessions = BTreeSet::new();
    let mut surfaces = BTreeSet::new();
    let mut relation_ids = BTreeSet::new();
    for row in valid {
        if relation_ids.insert(row.relation_id.clone()) {
            prefixes.insert(row.program_hint.prefix.clone());
            sessions.insert(row.source_session_id_sha256.clone());
            surfaces.insert(row.surface_id_sha256.clone());
        }
    }
    let mut packages = Vec::new();
    if relation_ids.len() >= 32 {
        let package_id = package_id(&prefixes);
        let package_shadows = shadows
            .iter()
            .filter(|row| {
                row.schema == "nando.response-shadow-observation.v1" && row.package_id == package_id
            })
            .collect::<Vec<_>>();
        let future_rows = package_shadows
            .iter()
            .filter(|row| row.matched_guard)
            .count();
        let wrong_accepts = package_shadows
            .iter()
            .filter(|row| row.matched_guard && !row.verifier_ok)
            .count();
        let future_sessions = package_shadows
            .iter()
            .map(|row| row.source_session_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let future_surfaces = package_shadows
            .iter()
            .map(|row| row.surface_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let state = if future_rows >= 32
            && wrong_accepts == 0
            && future_sessions.len() >= 3
            && future_surfaces.len() >= 2
            && wave_causal_pass
        {
            ResponsePackageState::Active
        } else {
            ResponsePackageState::Quarantine
        };
        let phase_centers = prefixes
            .iter()
            .flat_map(|prefix| response_phase_atom_ids_for_prefix(prefix))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        packages.push(ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id,
            origin: ResponsePackageOrigin::LegacyTemplate,
            state,
            program: ResponseProgram::copy_after_prefix(prefixes),
            verifier: None,
            routing_predicates: Vec::new(),
            required_routing_atom_ids: Vec::new(),
            phase_centers,
            anti_centers: package_shadows
                .iter()
                .filter(|row| row.matched_guard && !row.verifier_ok)
                .map(|row| phase_word(&row.source_session_id_sha256))
                .collect(),
            wave_margin_micro: if wave_causal_pass { 850_000 } else { 1 },
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: relation_ids.len(),
                future_rows,
                distinct_sessions: sessions.len().max(future_sessions.len()),
                distinct_surfaces: surfaces.len().max(future_surfaces.len()),
                wrong_accepts,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass,
                verifier_schema: "response_actor_independent_verifier.v1".to_owned(),
            },
        });
    }
    let mut wait_rows = relations
        .iter()
        .filter(|row| {
            row.schema == "nando.response-relation-observation.v1"
                && row.relation == "yielded_cell_to_wait_function_call"
                && row.program_hint.op == "wait_on_yielded_cell"
                && row.guard_schema == "wait_long_running_batch_guard.v5"
        })
        .collect::<Vec<_>>();
    wait_rows.sort_by(|left, right| {
        left.observed_at
            .cmp(&right.observed_at)
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    if wait_rows.len() >= 64 {
        let support = &wait_rows[..32];
        let future = &wait_rows[32..];
        let wrong_accepts = future.iter().filter(|row| !row.verifier_ok).count();
        let sessions = wait_rows
            .iter()
            .map(|row| row.source_session_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let surfaces = wait_rows
            .iter()
            .map(|row| row.surface_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let future_sessions = future
            .iter()
            .map(|row| row.source_session_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let future_surfaces = future
            .iter()
            .map(|row| row.surface_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let active = wrong_accepts == 0
            && future.len() >= 32
            && future_sessions.len() >= 3
            && future_surfaces.len() >= 2
            && wave_causal_pass;
        packages.push(ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "raw-phase-wait-on-yielded-cell-v1".to_owned(),
            origin: ResponsePackageOrigin::LegacyTemplate,
            state: if active {
                ResponsePackageState::Active
            } else {
                ResponsePackageState::Quarantine
            },
            program: ResponseProgram::wait_on_yielded_cell(),
            verifier: None,
            routing_predicates: Vec::new(),
            required_routing_atom_ids: Vec::new(),
            phase_centers: crate::package::response_phase_atom_ids_for_wait(),
            anti_centers: future
                .iter()
                .filter(|row| !row.verifier_ok)
                .map(|row| phase_word(&row.relation_id))
                .collect(),
            wave_margin_micro: 850_000,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: support.len(),
                future_rows: future.len(),
                distinct_sessions: sessions.len(),
                distinct_surfaces: surfaces.len(),
                wrong_accepts,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass,
                verifier_schema: "response_actor_independent_verifier.v1".to_owned(),
            },
        });
    }
    let mut any_wait_rows = relations
        .iter()
        .filter(|row| {
            row.schema == "nando.response-relation-observation.v1"
                && row.relation == "yielded_cell_to_any_wait_function_call"
                && row.program_hint.op == "wait_on_any_yielded_cell"
                && row.guard_schema == "wait_any_yielded_cell_guard.v1"
        })
        .collect::<Vec<_>>();
    any_wait_rows.sort_by(|left, right| {
        left.observed_at
            .cmp(&right.observed_at)
            .then_with(|| left.relation_id.cmp(&right.relation_id))
    });
    if any_wait_rows.len() >= 64 {
        let support = &any_wait_rows[..32];
        let future = &any_wait_rows[32..];
        let wrong_accepts = future.iter().filter(|row| !row.verifier_ok).count();
        let sessions = any_wait_rows
            .iter()
            .map(|row| row.source_session_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let surfaces = any_wait_rows
            .iter()
            .map(|row| row.surface_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let future_sessions = future
            .iter()
            .map(|row| row.source_session_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let future_surfaces = future
            .iter()
            .map(|row| row.surface_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let active = wrong_accepts == 0
            && future.len() >= 32
            && future_sessions.len() >= 3
            && future_surfaces.len() >= 2
            && wave_causal_pass;
        packages.push(ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: "raw-phase-wait-on-any-yielded-cell-v1".to_owned(),
            origin: ResponsePackageOrigin::LegacyTemplate,
            state: if active {
                ResponsePackageState::Active
            } else {
                ResponsePackageState::Quarantine
            },
            program: ResponseProgram::wait_on_any_yielded_cell(),
            verifier: None,
            routing_predicates: Vec::new(),
            required_routing_atom_ids: Vec::new(),
            phase_centers: crate::package::response_phase_atom_ids_for_any_wait(),
            anti_centers: future
                .iter()
                .filter(|row| !row.verifier_ok)
                .map(|row| phase_word(&row.relation_id))
                .collect(),
            wave_margin_micro: 850_000,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: support.len(),
                future_rows: future.len(),
                distinct_sessions: sessions.len(),
                distinct_surfaces: surfaces.len(),
                wrong_accepts,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass,
                verifier_schema: "response_actor_independent_verifier.v1".to_owned(),
            },
        });
    }
    let mut surface_groups: BTreeMap<&str, Vec<&ResponseRelationObservation>> = BTreeMap::new();
    for row in relations.iter().filter(|row| {
        row.schema == "nando.response-relation-observation.v1"
            && row.relation == "yielded_cell_to_surface_wait_function_call"
            && row.program_hint.op == "wait_on_yielded_surfaces"
            && row.guard_schema == "wait_yielded_surface_guard.v3"
            && !row.program_hint.prefix.is_empty()
    }) {
        surface_groups
            .entry(row.program_hint.prefix.as_str())
            .or_default()
            .push(row);
    }
    for (surface, mut rows) in surface_groups {
        rows.sort_by(|left, right| {
            left.observed_at
                .cmp(&right.observed_at)
                .then_with(|| left.relation_id.cmp(&right.relation_id))
        });
        if rows.len() < 16 {
            continue;
        }
        let support_len = rows.len().min(32);
        let support = &rows[..support_len];
        let future = &rows[support_len..];
        let wrong_accepts = rows.iter().filter(|row| !row.verifier_ok).count();
        let sessions = rows
            .iter()
            .map(|row| row.source_session_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let future_sessions = future
            .iter()
            .map(|row| row.source_session_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let surfaces = rows
            .iter()
            .map(|row| row.surface_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let future_surfaces = future
            .iter()
            .map(|row| row.surface_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let active = wrong_accepts == 0
            && support.len() >= 32
            && future.len() >= 32
            && future_sessions.len() >= 3
            && future_surfaces.len() >= 2
            && wave_causal_pass;
        packages.push(ResponsePackage {
            schema: "nando.response-package.v1".to_owned(),
            package_id: format!("raw-phase-wait-surface-{:016x}", phase_word(surface)),
            origin: ResponsePackageOrigin::LegacyTemplate,
            state: if active {
                ResponsePackageState::Active
            } else {
                ResponsePackageState::Quarantine
            },
            program: ResponseProgram::wait_on_yielded_surfaces([surface]),
            verifier: None,
            routing_predicates: Vec::new(),
            required_routing_atom_ids: Vec::new(),
            phase_centers: crate::package::response_phase_atom_ids_for_wait_surface(surface),
            anti_centers: future
                .iter()
                .filter(|row| !row.verifier_ok)
                .map(|row| phase_word(&row.relation_id))
                .collect(),
            wave_margin_micro: 850_000,
            learned_wave_route: None,
            crystallized_operator: None,
            proof: ResponsePackageProof {
                support_rows: support.len(),
                future_rows: future.len(),
                distinct_sessions: sessions.len(),
                distinct_surfaces: surfaces.len(),
                wrong_accepts,
                runtime_parity_failures: 0,
                exact_cache_overlap: 0,
                wave_causal_pass,
                verifier_schema: "response_actor_independent_verifier.v1".to_owned(),
            },
        });
    }
    ResponseRegistry {
        schema: "nando.response-registry.v5".to_owned(),
        revision,
        packages,
    }
}

fn package_id(prefixes: &BTreeSet<String>) -> String {
    let joined = prefixes.iter().cloned().collect::<Vec<_>>().join("\0");
    format!("raw-phase-copy-{:016x}", phase_word(&joined))
}

fn phase_word(value: &str) -> u64 {
    value.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cardinality_frame(output_count: u32, message_count: u32, label: bool) -> RelationFrame {
        RelationFrame {
            schema: crate::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: format!("{output_count:064x}"),
            event_id_sha256: "1".repeat(64),
            client_intent_id_sha256: "2".repeat(64),
            session_id_sha256: "3".repeat(64),
            observed_at_unix_nanos: u64::from(output_count),
            estimated_input_tokens: 1,
            extractor_version: crate::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(label),
            atoms: vec![
                RelationAtom::Cardinality {
                    role: "turn_output_count_band".to_owned(),
                    count: output_count,
                },
                RelationAtom::Cardinality {
                    role: "turn_message_count_band".to_owned(),
                    count: message_count,
                },
            ],
            evidence_ref_sha256: "4".repeat(64),
        }
    }

    #[test]
    fn routing_refinement_does_not_freeze_exact_conversation_age() {
        let support = (0..32)
            .map(|_| cardinality_frame(64, 8, true))
            .collect::<Vec<_>>();
        let negatives = (0..32)
            .map(|_| cardinality_frame(8, 0, false))
            .collect::<Vec<_>>();

        let refinement = select_clean_routing_refinement(&support, &negatives, &[]);
        assert!(refinement.exact_atom_ids.is_empty());
        assert_eq!(refinement.predicates.len(), 1);
        assert!(refinement.matches_frame(&cardinality_frame(128, 16, true)));
        assert!(!refinement.matches_frame(&cardinality_frame(8, 0, false)));
    }

    #[test]
    fn routing_refinement_synthesizes_a_conjunction_when_factors_overlap() {
        let support = (0..32)
            .map(|_| cardinality_frame(64, 8, true))
            .collect::<Vec<_>>();
        let mut negatives = (0..16)
            .map(|_| cardinality_frame(64, 0, false))
            .collect::<Vec<_>>();
        negatives.extend((0..16).map(|_| cardinality_frame(8, 8, false)));

        let refinement = select_clean_routing_refinement(&support, &negatives, &[]);
        assert!(refinement.exact_atom_ids.is_empty());
        assert_eq!(refinement.predicates.len(), 2);
        assert!(refinement.matches_frame(&cardinality_frame(128, 16, true)));
        assert!(!refinement.matches_frame(&cardinality_frame(64, 0, false)));
        assert!(!refinement.matches_frame(&cardinality_frame(8, 8, false)));
    }

    #[test]
    fn typed_action_contract_ignores_budgets_but_requires_semantic_constants() {
        let program = ResponseProgram::function_call_from_roles(
            "wait",
            crate::ResponseValueSelector::ContentLinePrefix {
                prefix: "Script running with cell ID ".to_owned(),
                value_type: crate::AtomValueType::Identifier,
            },
            vec![
                ResponseArgument::Role {
                    name: "cell_id".to_owned(),
                    role: crate::SemanticRole::ContinuationHandle,
                    value_type: None,
                },
                ResponseArgument::Integer {
                    name: "yield_time_ms".to_owned(),
                    value: 10_000,
                },
                ResponseArgument::Integer {
                    name: "max_tokens".to_owned(),
                    value: 1_000,
                },
                ResponseArgument::Integer {
                    name: "attempt".to_owned(),
                    value: 3,
                },
            ],
        );
        let mut frame = cardinality_frame(8, 1, false);
        frame.atoms.extend([
            RelationAtom::CompletionState {
                value: "pending".to_owned(),
            },
            RelationAtom::ResponseShape {
                value: "function_call".to_owned(),
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
                name: "yield_time_ms".to_owned(),
                value: 10_000,
            },
            RelationAtom::ActionIntegerArgument {
                name: "max_tokens".to_owned(),
                value: 1_000,
            },
            RelationAtom::ActionIntegerArgument {
                name: "attempt".to_owned(),
                value: 3,
            },
            RelationAtom::TypedSlot {
                slot_id: 1,
                source: crate::AtomSource::Observation,
                value_type: crate::AtomValueType::Identifier,
                value_sha256: "5".repeat(64),
            },
            RelationAtom::TypedSlot {
                slot_id: 2,
                source: crate::AtomSource::Action,
                value_type: crate::AtomValueType::Identifier,
                value_sha256: "5".repeat(64),
            },
            RelationAtom::SlotEquality {
                left_slot: 1,
                right_slot: 2,
            },
        ]);

        assert!(frame_matches_program_action_contract(&program, &frame));
        let mut mismatched = frame.clone();
        if let Some(RelationAtom::ActionIntegerArgument { value, .. }) = mismatched
            .atoms
            .iter_mut()
            .find(|atom| matches!(atom, RelationAtom::ActionIntegerArgument { name, .. } if name == "max_tokens"))
        {
            *value = 999;
        }
        assert!(frame_matches_program_action_contract(&program, &mismatched));
        if let Some(RelationAtom::ActionIntegerArgument { value, .. }) = mismatched
            .atoms
            .iter_mut()
            .find(|atom| matches!(atom, RelationAtom::ActionIntegerArgument { name, .. } if name == "attempt"))
        {
            *value = 4;
        }
        assert!(!frame_matches_program_action_contract(
            &program,
            &mismatched
        ));
    }

    #[test]
    fn project_status_lifecycle_uses_external_schema_and_family_identity() {
        let selector = crate::ResponseValueSelector::JsonField {
            field: "exit_code".to_owned(),
            value_type: crate::AtomValueType::Integer,
        };
        let program = ResponseProgram::project_status(
            selector,
            crate::ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        assert_eq!(
            response_program_external_verifier_schema(&program),
            Some("status_projection_external_evidence.v1")
        );
        let required = response_program_required_routing_atom_ids(&program);
        assert_ne!(
            family_lineage_id(1, &program, &required),
            family_lineage_id(2, &program, &required)
        );
    }
}
