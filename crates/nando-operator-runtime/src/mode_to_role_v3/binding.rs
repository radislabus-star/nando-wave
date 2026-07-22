use nando_core::wave::{RuntimeRoleBinder, SearchCompletion};

use super::encoding::runtime_candidate_bundle_v3;
use super::{
    F5C_MAX_MAPPING_EVALUATIONS_V3, F5C_MAX_MAPPINGS_PER_MODE_V3,
    F5C_MAX_SOURCE_CANDIDATE_EVALUATIONS_V3, ModeStructuralBindingReportV3,
    RuntimeStructuralMappingV3, StructuralBindingOutcomeV3, StructuralBindingVerdictV3,
    StructuralDispatchIndexV3, StructuralDispatchReportV3, StructuralDispatchVerdictV3,
};
use crate::CanonicalRuntimeRequestV3;

#[must_use]
pub fn bind_structural_modes_v3(
    index: &StructuralDispatchIndexV3,
    request: &CanonicalRuntimeRequestV3<'_>,
    dispatch: &StructuralDispatchReportV3,
) -> StructuralBindingOutcomeV3 {
    if dispatch.index_sha256 != index.index_sha256 {
        return blocked(
            index,
            request,
            0,
            0,
            StructuralBindingVerdictV3::RejectIndexMismatch,
        );
    }
    if dispatch.verdict == StructuralDispatchVerdictV3::AbstainDispatchExhausted {
        return blocked(
            index,
            request,
            0,
            0,
            StructuralBindingVerdictV3::AbstainDispatchExhausted,
        );
    }

    let mut total_source_candidates = 0_usize;
    let mut total_mappings = 0_usize;
    let mut mode_reports = Vec::with_capacity(dispatch.mode_indices.len());
    for mode_index in dispatch.mode_indices.iter().copied() {
        let Some(mode) = index.modes.get(mode_index) else {
            return blocked(
                index,
                request,
                total_source_candidates,
                total_mappings,
                StructuralBindingVerdictV3::RejectIndexMismatch,
            );
        };
        let mut mappings = Vec::new();
        let mut source_candidate_evaluations = 0_usize;
        for candidate in request
            .view()
            .structural
            .roles
            .iter()
            .filter(|role| role.features.value_type == mode.source_value_type)
        {
            if total_source_candidates >= F5C_MAX_SOURCE_CANDIDATE_EVALUATIONS_V3 {
                return blocked(
                    index,
                    request,
                    total_source_candidates,
                    total_mappings,
                    StructuralBindingVerdictV3::AbstainBudgetExhausted,
                );
            }
            total_source_candidates = total_source_candidates.saturating_add(1);
            source_candidate_evaluations = source_candidate_evaluations.saturating_add(1);
            let Ok(bundle) = runtime_candidate_bundle_v3(mode, candidate, request.request_sha256())
            else {
                return blocked(
                    index,
                    request,
                    total_source_candidates,
                    total_mappings,
                    StructuralBindingVerdictV3::AbstainBindingExhausted,
                );
            };
            let remaining_mode = F5C_MAX_MAPPINGS_PER_MODE_V3.saturating_sub(mappings.len());
            let remaining_global = F5C_MAX_MAPPING_EVALUATIONS_V3.saturating_sub(total_mappings);
            if remaining_mode == 0 {
                return blocked(
                    index,
                    request,
                    total_source_candidates,
                    total_mappings,
                    StructuralBindingVerdictV3::AbstainBindingExhausted,
                );
            }
            if remaining_global == 0 {
                return blocked(
                    index,
                    request,
                    total_source_candidates,
                    total_mappings,
                    StructuralBindingVerdictV3::AbstainBudgetExhausted,
                );
            }
            let remaining = remaining_mode.min(remaining_global);
            let report = RuntimeRoleBinder::bind(
                &mode.role_graph,
                &mode.relation_program,
                &bundle,
                remaining,
            );
            if !matches!(report.completion(), SearchCompletion::Complete { .. }) {
                return blocked(
                    index,
                    request,
                    total_source_candidates,
                    total_mappings,
                    StructuralBindingVerdictV3::AbstainBindingExhausted,
                );
            }
            total_mappings = total_mappings.saturating_add(report.structural_mappings().len());
            mappings.extend(report.structural_mappings().iter().map(|mapping| {
                RuntimeStructuralMappingV3 {
                    runtime_source_role_id: candidate.role_id,
                    local_to_canonical: mapping.local_to_canonical().to_vec().into_boxed_slice(),
                    phase_fit_fixed: mapping.phase_fit_fixed(),
                    phase_components_fixed: mapping.phase_components_fixed().into(),
                }
            }));
        }
        mappings.sort_by(|left, right| {
            right
                .phase_fit_fixed
                .cmp(&left.phase_fit_fixed)
                .then_with(|| {
                    left.runtime_source_role_id
                        .cmp(&right.runtime_source_role_id)
                        .then_with(|| left.local_to_canonical.cmp(&right.local_to_canonical))
                })
        });
        let phase_winner_count = mappings.first().map_or(0, |winner| {
            mappings
                .iter()
                .take_while(|mapping| mapping.phase_fit_fixed == winner.phase_fit_fixed)
                .count()
        });
        let phase_runner_up_fit_fixed = mappings
            .get(phase_winner_count)
            .map(|mapping| mapping.phase_fit_fixed);
        mode_reports.push(ModeStructuralBindingReportV3 {
            mode_id_sha256: mode.mode_id_sha256.clone(),
            mappings: mappings.into_boxed_slice(),
            source_candidate_evaluations,
            phase_winner_count,
            phase_runner_up_fit_fixed,
        });
    }

    StructuralBindingOutcomeV3 {
        index_sha256: index.index_sha256.clone(),
        request_view_sha256: request.view().request_view_sha256.clone(),
        mode_reports: mode_reports.into_boxed_slice(),
        source_candidate_evaluations: total_source_candidates,
        mapping_evaluations: total_mappings,
        verdict: StructuralBindingVerdictV3::Complete,
    }
}

fn blocked(
    index: &StructuralDispatchIndexV3,
    request: &CanonicalRuntimeRequestV3<'_>,
    source_candidate_evaluations: usize,
    mapping_evaluations: usize,
    verdict: StructuralBindingVerdictV3,
) -> StructuralBindingOutcomeV3 {
    StructuralBindingOutcomeV3 {
        index_sha256: index.index_sha256.clone(),
        request_view_sha256: request.view().request_view_sha256.clone(),
        mode_reports: Box::new([]),
        source_candidate_evaluations,
        mapping_evaluations,
        verdict,
    }
}
