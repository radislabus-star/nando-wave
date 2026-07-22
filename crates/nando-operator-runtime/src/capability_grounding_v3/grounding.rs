use std::collections::{BTreeMap, BTreeSet};

use super::derivation::{
    capability_matches_mode_v3, derive_action_v3, mapping_digest_v3, missing_capability_attempt_v3,
};
use super::{
    ActionDerivationVerdictV3, BoundProtocolActionOutcomeV3, CapabilityGroundingVerdictV3,
    F5D_MAX_ACTION_DERIVATIONS_V3,
};
use crate::{
    CanonicalRuntimeRequestV3, CompleteRuntimeRoleBindingReportV3, StructuralDispatchIndexV3,
};

#[must_use]
pub fn ground_protocol_actions_v3(
    index: &StructuralDispatchIndexV3,
    request: &CanonicalRuntimeRequestV3<'_>,
    binding: &CompleteRuntimeRoleBindingReportV3,
) -> BoundProtocolActionOutcomeV3 {
    if binding.index_sha256() != index.index_sha256()
        || binding.request_view_sha256() != request.view().request_view_sha256
    {
        return outcome(
            index,
            request,
            Vec::new(),
            Vec::new(),
            0,
            0,
            CapabilityGroundingVerdictV3::RejectIndexMismatch,
        );
    }

    let modes = index
        .modes()
        .iter()
        .map(|mode| (mode.mode_id_sha256(), mode))
        .collect::<BTreeMap<_, _>>();
    let mut attempts = Vec::new();
    let mut actions = Vec::new();
    let mut structural_mappings = 0_usize;
    let mut action_derivations = 0_usize;
    let mut every_mapping_bound = true;

    for report in binding.mode_reports() {
        let Some(mode) = modes.get(report.mode_id_sha256()).copied() else {
            return outcome(
                index,
                request,
                attempts,
                actions,
                structural_mappings,
                action_derivations,
                CapabilityGroundingVerdictV3::RejectIndexMismatch,
            );
        };
        for mapping in report.mappings() {
            structural_mappings = structural_mappings.saturating_add(1);
            let Ok(mapping_sha256) = mapping_digest_v3(mode, mapping) else {
                return outcome(
                    index,
                    request,
                    attempts,
                    actions,
                    structural_mappings,
                    action_derivations,
                    CapabilityGroundingVerdictV3::AbstainRoleValue,
                );
            };
            let matching = request
                .view()
                .capabilities
                .iter()
                .filter(|capability| capability_matches_mode_v3(mode, capability))
                .filter_map(|descriptor| {
                    request
                        .capability_bindings()
                        .iter()
                        .find(|binding| binding.capability_id == descriptor.capability_id)
                })
                .collect::<Vec<_>>();
            if matching.is_empty() {
                attempts.push(missing_capability_attempt_v3(
                    mode,
                    mapping,
                    &mapping_sha256,
                ));
                every_mapping_bound = false;
                continue;
            }
            let mut mapping_bound = false;
            for capability in matching {
                if action_derivations >= F5D_MAX_ACTION_DERIVATIONS_V3 {
                    return outcome(
                        index,
                        request,
                        attempts,
                        actions,
                        structural_mappings,
                        action_derivations,
                        CapabilityGroundingVerdictV3::AbstainBudgetExhausted,
                    );
                }
                action_derivations = action_derivations.saturating_add(1);
                let derived = derive_action_v3(
                    index.index_sha256(),
                    request,
                    mode,
                    mapping,
                    &mapping_sha256,
                    capability,
                );
                mapping_bound |= derived.action.is_some();
                attempts.push(derived.attempt);
                if let Some(action) = derived.action {
                    actions.push(action);
                }
            }
            every_mapping_bound &= mapping_bound;
        }
    }

    let semantic_classes = actions
        .iter()
        .map(|action| action.semantic_action_sha256())
        .collect::<BTreeSet<_>>()
        .len();
    let physical_classes = actions
        .iter()
        .map(|action| action.physical_action_sha256())
        .collect::<BTreeSet<_>>()
        .len();
    let has_ambiguous_capability = attempts
        .iter()
        .any(|attempt| attempt.verdict == ActionDerivationVerdictV3::AmbiguousCapabilityTopology);
    let has_invalid_derivation = attempts.iter().any(|attempt| {
        !matches!(
            attempt.verdict,
            ActionDerivationVerdictV3::Bound
                | ActionDerivationVerdictV3::MissingCapability
                | ActionDerivationVerdictV3::AmbiguousCapabilityTopology
        )
    });
    let verdict = if structural_mappings == 0 {
        CapabilityGroundingVerdictV3::AbstainNoStructuralMapping
    } else if has_ambiguous_capability {
        CapabilityGroundingVerdictV3::AbstainAmbiguousCapability
    } else if has_invalid_derivation {
        CapabilityGroundingVerdictV3::AbstainRoleValue
    } else if !every_mapping_bound {
        verdict_for_unbound(&attempts)
    } else if semantic_classes > 1 {
        CapabilityGroundingVerdictV3::AbstainAmbiguousAction
    } else if physical_classes > 1 {
        CapabilityGroundingVerdictV3::AbstainAmbiguousCapability
    } else if semantic_classes == 1 && physical_classes == 1 {
        CapabilityGroundingVerdictV3::Complete
    } else {
        CapabilityGroundingVerdictV3::AbstainMissingCapability
    };
    outcome(
        index,
        request,
        attempts,
        actions,
        structural_mappings,
        action_derivations,
        verdict,
    )
}

fn verdict_for_unbound(attempts: &[super::MappingActionAttemptV3]) -> CapabilityGroundingVerdictV3 {
    if attempts
        .iter()
        .any(|attempt| attempt.verdict == ActionDerivationVerdictV3::AmbiguousCapabilityTopology)
    {
        CapabilityGroundingVerdictV3::AbstainAmbiguousCapability
    } else if attempts
        .iter()
        .any(|attempt| attempt.verdict == ActionDerivationVerdictV3::MissingCapability)
    {
        CapabilityGroundingVerdictV3::AbstainMissingCapability
    } else {
        CapabilityGroundingVerdictV3::AbstainRoleValue
    }
}

#[allow(clippy::too_many_arguments)]
fn outcome(
    index: &StructuralDispatchIndexV3,
    request: &CanonicalRuntimeRequestV3<'_>,
    attempts: Vec<super::MappingActionAttemptV3>,
    mut actions: Vec<nando_operator_kernel::BoundProtocolActionV3>,
    structural_mappings: usize,
    action_derivations: usize,
    verdict: CapabilityGroundingVerdictV3,
) -> BoundProtocolActionOutcomeV3 {
    actions.sort_by(|left, right| {
        left.physical_action_sha256()
            .cmp(right.physical_action_sha256())
            .then_with(|| left.derivation_sha256().cmp(right.derivation_sha256()))
    });
    actions.dedup_by(|left, right| left.physical_action_sha256() == right.physical_action_sha256());
    let semantic_action_classes = actions
        .iter()
        .map(|action| action.semantic_action_sha256())
        .collect::<BTreeSet<_>>()
        .len();
    let physical_action_classes = actions.len();
    BoundProtocolActionOutcomeV3 {
        index_sha256: index.index_sha256().to_owned(),
        request_view_sha256: request.view().request_view_sha256.clone(),
        attempts: attempts.into_boxed_slice(),
        actions: actions.into_boxed_slice(),
        structural_mappings,
        action_derivations,
        semantic_action_classes,
        physical_action_classes,
        verdict,
    }
}
