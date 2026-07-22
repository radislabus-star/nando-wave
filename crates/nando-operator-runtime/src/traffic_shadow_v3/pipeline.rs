use std::sync::Arc;

use nando_operator_kernel::{RuntimeContextExtractionVerdictV3, RuntimeProjectionV3};

use super::receipt::TrafficShadowReceiptBuilderV3;
use super::{
    TrafficShadowExecutionV3, TrafficShadowGenerationV3, TrafficShadowInputV3,
    TrafficShadowReceiptV3, TrafficShadowVerdictV3,
};
use crate::{
    CapabilityGroundingVerdictV3, OperatorShadowVerdictV3, PhaseControlV3, PhaseSelectionVerdictV3,
    RuntimeContextBudgetV3, RuntimeContextErrorV3, StructuralBindingVerdictV3,
    StructuralDispatchVerdictV3, bind_structural_modes_v3, evaluate_phase_ranking_v3,
    execute_bound_protocol_shadow_v3, export_runtime_phase_control_evidence_v3,
    extract_canonical_runtime_request_v3, ground_protocol_actions_v3,
};

#[must_use]
pub fn execute_traffic_shadow_v3(
    generation: Arc<TrafficShadowGenerationV3>,
    input: TrafficShadowInputV3<'_>,
    budget: RuntimeContextBudgetV3,
) -> TrafficShadowReceiptV3 {
    execute_traffic_shadow_with_handoff_v3(generation, input, budget).into_receipt()
}

#[must_use]
pub fn execute_traffic_shadow_with_handoff_v3(
    generation: Arc<TrafficShadowGenerationV3>,
    input: TrafficShadowInputV3<'_>,
    budget: RuntimeContextBudgetV3,
) -> TrafficShadowExecutionV3 {
    let mut receipt = TrafficShadowReceiptBuilderV3::new(&generation, &input);
    let Some((request_text, provider_payload)) = input.runtime_payload() else {
        return finish(receipt, TrafficShadowVerdictV3::CensoredPayloadUnavailable);
    };
    let Some(projection) = supported_projection(input.projection()) else {
        return finish(
            receipt,
            TrafficShadowVerdictV3::AbstainUnsupportedProjection,
        );
    };
    let extraction = match extract_canonical_runtime_request_v3(
        input.request_sha256(),
        request_text,
        projection,
        provider_payload,
        budget,
    ) {
        Ok(extraction) => extraction,
        Err(error) => return finish(receipt, context_error_verdict(error)),
    };
    receipt.set_extraction_receipt(&extraction.receipt().receipt_sha256);
    if extraction.receipt().verdict != RuntimeContextExtractionVerdictV3::Complete {
        return finish(
            receipt,
            match extraction.receipt().verdict {
                RuntimeContextExtractionVerdictV3::AbstainBudgetExhausted => {
                    TrafficShadowVerdictV3::AbstainContextBudget
                }
                RuntimeContextExtractionVerdictV3::AbstainInvalidRequest => {
                    TrafficShadowVerdictV3::AbstainContextExtraction
                }
                RuntimeContextExtractionVerdictV3::Complete => {
                    TrafficShadowVerdictV3::RejectInvariantMismatch
                }
            },
        );
    }
    let Some(context) = extraction.into_context() else {
        return finish(receipt, TrafficShadowVerdictV3::RejectInvariantMismatch);
    };

    let dispatch = generation.index().dispatch(&context);
    if dispatch.verdict() != StructuralDispatchVerdictV3::Complete
        || dispatch.mode_indices().is_empty()
    {
        return finish(receipt, TrafficShadowVerdictV3::AbstainDispatch);
    }
    let binding = bind_structural_modes_v3(generation.index(), &context, &dispatch);
    if binding.verdict() != StructuralBindingVerdictV3::Complete {
        return finish(receipt, binding_verdict(binding.verdict()));
    }
    let Some(binding) = binding.into_complete() else {
        return finish(receipt, TrafficShadowVerdictV3::RejectInvariantMismatch);
    };

    let grounded = ground_protocol_actions_v3(generation.index(), &context, &binding);
    let phase = evaluate_phase_ranking_v3(&grounded);
    receipt.set_phase_report(phase.report_sha256());
    let phase_evidence = match export_runtime_phase_control_evidence_v3(&phase) {
        Ok(evidence) => evidence,
        Err(_) => return finish(receipt, TrafficShadowVerdictV3::RejectInvariantMismatch),
    };
    if grounded.verdict() != CapabilityGroundingVerdictV3::Complete {
        return finish_with_phase(
            receipt,
            grounding_verdict(grounded.verdict()),
            phase_evidence,
        );
    }
    let full_phase = phase
        .controls()
        .iter()
        .find(|control| control.control() == PhaseControlV3::Full);
    let Some(actions) = grounded.into_complete() else {
        return finish_with_phase(
            receipt,
            TrafficShadowVerdictV3::RejectInvariantMismatch,
            phase_evidence,
        );
    };
    if full_phase.is_none_or(|control| {
        control.verdict() != PhaseSelectionVerdictV3::Selected
            || control.selected_physical_action_sha256()
                != Some(actions.action().physical_action_sha256())
    }) {
        return finish_with_phase(
            receipt,
            TrafficShadowVerdictV3::AbstainPhase,
            phase_evidence,
        );
    }

    let shadow = execute_bound_protocol_shadow_v3(actions.action());
    receipt.set_operator_shadow_receipt(shadow.receipt().receipt_sha256());
    let verdict = match shadow.receipt().verdict() {
        OperatorShadowVerdictV3::Complete => TrafficShadowVerdictV3::CompleteShadow,
        OperatorShadowVerdictV3::ParityMismatch => TrafficShadowVerdictV3::ActorVmParityMismatch,
        _ => TrafficShadowVerdictV3::AbstainActorVm,
    };
    if verdict == TrafficShadowVerdictV3::CompleteShadow
        && let Some(actor_output) = shadow.actor_output()
    {
        return TrafficShadowExecutionV3::complete(
            receipt.finish(verdict),
            actions.action().clone(),
            actor_output.to_owned(),
            phase_evidence,
        );
    }
    finish_with_phase(receipt, verdict, phase_evidence)
}

fn finish(
    receipt: TrafficShadowReceiptBuilderV3,
    verdict: TrafficShadowVerdictV3,
) -> TrafficShadowExecutionV3 {
    TrafficShadowExecutionV3::receipt_only(receipt.finish(verdict))
}

fn finish_with_phase(
    receipt: TrafficShadowReceiptBuilderV3,
    verdict: TrafficShadowVerdictV3,
    phase_control_evidence: nando_operator_kernel::RuntimePhaseControlEvidenceV3,
) -> TrafficShadowExecutionV3 {
    TrafficShadowExecutionV3::receipt_with_phase(receipt.finish(verdict), phase_control_evidence)
}

const fn supported_projection(
    projection: Option<RuntimeProjectionV3>,
) -> Option<RuntimeProjectionV3> {
    match projection {
        Some(
            projection @ (RuntimeProjectionV3::Responses | RuntimeProjectionV3::ChatCompletions),
        ) => Some(projection),
        None | Some(RuntimeProjectionV3::TransitionApi) => None,
    }
}

const fn context_error_verdict(error: RuntimeContextErrorV3) -> TrafficShadowVerdictV3 {
    match error {
        RuntimeContextErrorV3::InvalidBudget => TrafficShadowVerdictV3::AbstainContextBudget,
        RuntimeContextErrorV3::InvalidRequestDigest
        | RuntimeContextErrorV3::Structural
        | RuntimeContextErrorV3::Serialization => TrafficShadowVerdictV3::AbstainContextExtraction,
    }
}

const fn binding_verdict(verdict: StructuralBindingVerdictV3) -> TrafficShadowVerdictV3 {
    match verdict {
        StructuralBindingVerdictV3::RejectIndexMismatch => {
            TrafficShadowVerdictV3::RejectInvariantMismatch
        }
        StructuralBindingVerdictV3::AbstainDispatchExhausted => {
            TrafficShadowVerdictV3::AbstainDispatch
        }
        StructuralBindingVerdictV3::AbstainBudgetExhausted => {
            TrafficShadowVerdictV3::AbstainRuntimeBudget
        }
        StructuralBindingVerdictV3::Complete
        | StructuralBindingVerdictV3::AbstainBindingExhausted => {
            TrafficShadowVerdictV3::AbstainBinding
        }
    }
}

const fn grounding_verdict(verdict: CapabilityGroundingVerdictV3) -> TrafficShadowVerdictV3 {
    match verdict {
        CapabilityGroundingVerdictV3::RejectIndexMismatch => {
            TrafficShadowVerdictV3::RejectInvariantMismatch
        }
        CapabilityGroundingVerdictV3::AbstainMissingCapability => {
            TrafficShadowVerdictV3::AbstainMissingCapability
        }
        CapabilityGroundingVerdictV3::AbstainAmbiguousCapability => {
            TrafficShadowVerdictV3::AbstainAmbiguousCapability
        }
        CapabilityGroundingVerdictV3::AbstainAmbiguousAction => {
            TrafficShadowVerdictV3::AbstainAmbiguousAction
        }
        CapabilityGroundingVerdictV3::AbstainRoleValue => TrafficShadowVerdictV3::AbstainRoleValue,
        CapabilityGroundingVerdictV3::AbstainBudgetExhausted => {
            TrafficShadowVerdictV3::AbstainRuntimeBudget
        }
        CapabilityGroundingVerdictV3::Complete
        | CapabilityGroundingVerdictV3::AbstainNoStructuralMapping => {
            TrafficShadowVerdictV3::AbstainBinding
        }
    }
}
