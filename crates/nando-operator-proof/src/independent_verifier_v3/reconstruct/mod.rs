mod matching;
mod provenance;

use std::collections::BTreeSet;

use nando_operator_kernel::{
    BoundProtocolActionInputV3, BoundProtocolActionV3, CanonicalEffectLawV3,
    build_bound_protocol_action_v3, canonical_json_sha256,
};

pub(super) use self::matching::features_match_mode_v3;
use self::matching::{
    actor_matches_v3, capability_matches_mode_v3, independent_arguments_v3, role_matches_mode_v3,
};
use self::provenance::duplicate_capability_paths_v3;
use super::surface::{IndependentSurfaceV3, role_value_root_v3};
use super::{
    IndependentVerifierBudgetV3, IndependentVerifierErrorV3, IndependentVerifierInputV3,
    IndependentVerifierVerdictV3,
};

pub(super) struct ReconstructedActionV3 {
    pub action: BoundProtocolActionV3,
    pub law: CanonicalEffectLawV3,
    pub candidate_set_sha256: String,
    pub candidate_paths: usize,
}

pub(super) struct BlockedReconstructionV3 {
    pub candidate_set_sha256: Option<String>,
    pub candidate_paths: usize,
    pub action_classes: usize,
    pub verdict: IndependentVerifierVerdictV3,
}

pub(super) enum ReconstructionOutcomeV3 {
    Complete(Box<ReconstructedActionV3>),
    Blocked(BlockedReconstructionV3),
}

struct CandidatePathV3 {
    path_sha256: String,
    action: BoundProtocolActionV3,
    law: CanonicalEffectLawV3,
}

pub(super) fn reconstruct_action_v3(
    input: &IndependentVerifierInputV3<'_>,
    surface: &IndependentSurfaceV3,
    budget: IndependentVerifierBudgetV3,
) -> Result<ReconstructionOutcomeV3, IndependentVerifierErrorV3> {
    let mode_count = input.artifact_set().mode_count();
    if mode_count == 0 || mode_count > budget.max_modes {
        return Ok(blocked(
            None,
            0,
            0,
            IndependentVerifierVerdictV3::AbstainBudgetExhausted,
        ));
    }
    if duplicate_capability_paths_v3(&surface.capabilities)? {
        return Ok(blocked(
            None,
            0,
            0,
            IndependentVerifierVerdictV3::AbstainAmbiguousCandidate,
        ));
    }

    let mut candidates = Vec::new();
    let mut matching_capabilities = 0_usize;
    for entry in input.artifact_set().artifacts() {
        let artifact = entry.artifact();
        let law = entry.law();
        for (mode, executable) in artifact
            .source_mode_set()
            .modes
            .iter()
            .zip(artifact.modes())
        {
            let roles = surface
                .request_view
                .structural
                .roles
                .iter()
                .filter(|role| role_matches_mode_v3(role, mode))
                .collect::<Vec<_>>();
            for role in roles {
                let Some(values) = surface.role_values.get(&role.role_id) else {
                    continue;
                };
                if values.len() != 1 {
                    continue;
                }
                let role_value_root = role_value_root_v3(role.role_id, values)
                    .map_err(|_| IndependentVerifierErrorV3::Serialization)?;
                for capability in surface.capabilities.iter().filter(|capability| {
                    capability_matches_mode_v3(
                        capability,
                        executable.payload().capability_kind(),
                        executable.payload().arguments(),
                    )
                }) {
                    matching_capabilities = matching_capabilities.saturating_add(1);
                    if candidates.len() >= budget.max_candidate_paths {
                        return blocked_from_candidates(
                            &candidates,
                            IndependentVerifierVerdictV3::AbstainBudgetExhausted,
                        );
                    }
                    if capability.argument_topology_ambiguous {
                        return blocked_from_candidates(
                            &candidates,
                            IndependentVerifierVerdictV3::AbstainAmbiguousCandidate,
                        );
                    }
                    let Some(arguments) = independent_arguments_v3(mode, capability, &values[0])
                    else {
                        continue;
                    };
                    let mapping_sha256 = canonical_json_sha256(&(
                        "nando.f6.independent-role-mapping.v3",
                        mode.mode_id_sha256.as_str(),
                        role.role_id,
                        role_value_root.as_str(),
                    ))
                    .map_err(|_| IndependentVerifierErrorV3::Serialization)?;
                    let action = build_bound_protocol_action_v3(BoundProtocolActionInputV3 {
                        index_sha256: input.artifact_set().artifact_set_sha256().to_owned(),
                        artifact_root_sha256: artifact.artifact_sha256().to_owned(),
                        mode_id_sha256: mode.mode_id_sha256.clone(),
                        executable_mode_root_sha256: executable
                            .executable_mode_root_sha256()
                            .to_owned(),
                        payload_root_sha256: executable.payload().payload_root_sha256().to_owned(),
                        effect_law_id_sha256: mode.effect_law_id_sha256.clone(),
                        action_class_root_sha256: mode.action_class_root_sha256.clone(),
                        request_view_sha256: surface.request_view.request_view_sha256.clone(),
                        mapping_sha256,
                        capability_id: capability.capability_id,
                        capability_kind: capability.kind,
                        physical_symbol: capability.physical_symbol.clone(),
                        arguments,
                    })
                    .map_err(|_| IndependentVerifierErrorV3::Serialization)?;
                    let path_sha256 = canonical_json_sha256(&(
                        "nando.f6.independent-candidate-path.v3",
                        artifact.artifact_sha256(),
                        mode.mode_id_sha256.as_str(),
                        role.role_id,
                        capability.capability_id,
                        action.semantic_action_sha256(),
                        action.physical_action_sha256(),
                    ))
                    .map_err(|_| IndependentVerifierErrorV3::Serialization)?;
                    candidates.push(CandidatePathV3 {
                        path_sha256,
                        action,
                        law: law.clone(),
                    });
                }
            }
        }
    }
    finalize_candidates_v3(input, candidates, matching_capabilities)
}

fn finalize_candidates_v3(
    input: &IndependentVerifierInputV3<'_>,
    mut candidates: Vec<CandidatePathV3>,
    matching_capabilities: usize,
) -> Result<ReconstructionOutcomeV3, IndependentVerifierErrorV3> {
    candidates.sort_by(|left, right| left.path_sha256.cmp(&right.path_sha256));
    if candidates
        .windows(2)
        .any(|pair| pair[0].path_sha256 == pair[1].path_sha256)
    {
        return blocked_from_candidates(
            &candidates,
            IndependentVerifierVerdictV3::AbstainAmbiguousCandidate,
        );
    }
    let candidate_set_sha256 = candidate_set_root_v3(&candidates)?;
    if candidates.is_empty() {
        let verdict = if matching_capabilities == 0 {
            IndependentVerifierVerdictV3::AbstainMissingCapability
        } else {
            IndependentVerifierVerdictV3::AbstainAmbiguousCandidate
        };
        return Ok(blocked(Some(candidate_set_sha256), 0, 0, verdict));
    }
    let action_classes = action_class_count_v3(&candidates);
    if action_classes != 1 {
        return Ok(blocked(
            Some(candidate_set_sha256),
            candidates.len(),
            action_classes,
            IndependentVerifierVerdictV3::AbstainAmbiguousCandidate,
        ));
    }
    let candidate_paths = candidates.len();
    let Some(winner) = candidates
        .into_iter()
        .find(|candidate| actor_matches_v3(input.actor_action(), &candidate.action))
    else {
        return Ok(blocked(
            Some(candidate_set_sha256),
            candidate_paths,
            1,
            IndependentVerifierVerdictV3::RejectActorMutation,
        ));
    };
    Ok(ReconstructionOutcomeV3::Complete(Box::new(
        ReconstructedActionV3 {
            action: winner.action,
            law: winner.law,
            candidate_set_sha256,
            candidate_paths,
        },
    )))
}

fn candidate_set_root_v3(
    candidates: &[CandidatePathV3],
) -> Result<String, IndependentVerifierErrorV3> {
    let paths = candidates
        .iter()
        .map(|candidate| candidate.path_sha256.as_str())
        .collect::<Vec<_>>();
    canonical_json_sha256(&("nando.f6.independent-candidate-set.v3", paths))
        .map_err(|_| IndependentVerifierErrorV3::Serialization)
}

fn action_class_count_v3(candidates: &[CandidatePathV3]) -> usize {
    candidates
        .iter()
        .map(|candidate| candidate.action.physical_action_sha256())
        .collect::<BTreeSet<_>>()
        .len()
}

fn blocked_from_candidates(
    candidates: &[CandidatePathV3],
    verdict: IndependentVerifierVerdictV3,
) -> Result<ReconstructionOutcomeV3, IndependentVerifierErrorV3> {
    Ok(blocked(
        Some(candidate_set_root_v3(candidates)?),
        candidates.len(),
        action_class_count_v3(candidates),
        verdict,
    ))
}

fn blocked(
    candidate_set_sha256: Option<String>,
    candidate_paths: usize,
    action_classes: usize,
    verdict: IndependentVerifierVerdictV3,
) -> ReconstructionOutcomeV3 {
    ReconstructionOutcomeV3::Blocked(BlockedReconstructionV3 {
        candidate_set_sha256,
        candidate_paths,
        action_classes,
        verdict,
    })
}
