use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, K2InquiryObserverRequestV1,
    K2InquiryWorkerRequestV1,
};
use super::{
    K2_UNCERTAINTY_PLAN_DISPATCH_SCHEMA_V2, K2UncertaintyBatchPrecommitV2,
    K2UncertaintyCasePreverificationV2, K2UncertaintyPlanDispatchV2, K2UncertaintyPrivateCaseV1,
    K2UncertaintyPrivateSafetyDispositionV1, K2UncertaintyPublicCaseV1,
    K2UncertaintySafetyReceiptV1, K2UncertaintySafetyRequestV1, K2UncertaintyWorkspaceIdentityV2,
    blank_probe_dispatch_item_v2, denied_authority_v1, uncertainty_root_v1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyPlanSafetyBindingV2 {
    pub request: K2UncertaintySafetyRequestV1,
    pub receipt: K2UncertaintySafetyReceiptV1,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_self_formed_plan_dispatch_v2(
    batch: &K2UncertaintyBatchPrecommitV2,
    case: &K2UncertaintyCasePreverificationV2,
    public_case: &K2UncertaintyPublicCaseV1,
    private_case: &K2UncertaintyPrivateCaseV1,
    safety_bindings: Vec<K2UncertaintyPlanSafetyBindingV2>,
    worker_executable_sha256: &str,
    observer_executable_sha256: &str,
) -> K2CompositionResultV1<K2UncertaintyPlanDispatchV2> {
    batch.validate()?;
    case.validate()?;
    public_case.validate()?;
    private_case.validate()?;
    let plan = case
        .closure_plan
        .as_ref()
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_plan_dispatch_closure_unavailable",
        ))?;
    let entry = batch
        .cases
        .iter()
        .find(|value| value.case_id_sha256 == plan.case_id_sha256)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_plan_dispatch_case_missing",
        ))?;
    if !batch.dispatch_permitted
        || !entry.dispatchable
        || entry.case_preverification_root_sha256 != case.receipt_root_sha256
        || entry.closure_plan_root_sha256.as_deref() != Some(plan.plan_root_sha256.as_str())
        || batch.experiment_id_sha256 != public_case.vocabulary.experiment_id_sha256
        || private_case.experiment_id_sha256 != batch.experiment_id_sha256
        || private_case.public_case_root_sha256 != public_case.public_case_root_sha256
        || private_case.case_id_sha256 != plan.case_id_sha256
        || public_case.vocabulary.case_id_sha256 != plan.case_id_sha256
        || safety_bindings.len() as u64 != plan.plan_length
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_plan_dispatch_boundary_v2_invalid",
        ));
    }

    let planner = &case.closure_verification_request.planner_request;
    let mut items = Vec::with_capacity(safety_bindings.len());
    for (ordinal, binding) in safety_bindings.into_iter().enumerate() {
        binding.request.validate()?;
        binding.receipt.validate()?;
        let probe_root = &plan.ordered_probe_roots_sha256[ordinal];
        let selected_probe = planner
            .representatives
            .iter()
            .find(|value| &value.probe.probe_root_sha256 == probe_root)
            .map(|value| value.probe.clone())
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_plan_dispatch_probe_missing",
            ))?;
        let resolved_effect = private_case
            .mapping
            .iter()
            .find(|value| value.opaque_action_root_sha256 == selected_probe.action_id_sha256)
            .map(|value| value.effect.clone())
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_plan_dispatch_private_effect_missing",
            ))?;
        if binding.request.selection_root_sha256 != plan.plan_root_sha256
            || binding.request.selected_probe != selected_probe
            || binding.request.resolved_private_effect != resolved_effect
            || binding.request.vocabulary != public_case.vocabulary
            || binding.receipt.safety_request_root_sha256 != binding.request.request_root_sha256
            || binding.receipt.disposition != K2UncertaintyPrivateSafetyDispositionV1::Pass
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_plan_dispatch_safety_v2_invalid",
            ));
        }
        let worker_request = K2InquiryWorkerRequestV1::seal(
            plan.case_id_sha256.clone(),
            plan.plan_root_sha256.clone(),
            selected_probe.probe_root_sha256.clone(),
            selected_probe.action_id_sha256.clone(),
            worker_executable_sha256.to_owned(),
            selected_probe.initial_manifest.clone(),
            resolved_effect,
        )?;
        let observer_request = K2InquiryObserverRequestV1::seal(
            plan.case_id_sha256.clone(),
            selected_probe.probe_root_sha256.clone(),
            observer_executable_sha256.to_owned(),
        )?;
        let workspace_identity = K2UncertaintyWorkspaceIdentityV2::seal(
            plan.case_id_sha256.clone(),
            plan.plan_root_sha256.clone(),
            ordinal as u64,
        )?;
        let mut item = blank_probe_dispatch_item_v2(
            plan.case_id_sha256.clone(),
            plan.plan_root_sha256.clone(),
            ordinal as u64,
            selected_probe,
            binding.request,
            binding.receipt,
            worker_request,
            observer_request,
            workspace_identity,
        );
        item.reseal()?;
        items.push(item);
    }
    let workspace_roots = items
        .iter()
        .map(|value| value.workspace_identity.identity_root_sha256.as_str())
        .collect::<Vec<_>>();
    let workspace_denominator_root_sha256 = uncertainty_root_v1(&(
        "nando.k2-self-formed-workspace-denominator.v2",
        &plan.plan_root_sha256,
        workspace_roots,
    ))?;
    let mut dispatch = K2UncertaintyPlanDispatchV2 {
        schema: K2_UNCERTAINTY_PLAN_DISPATCH_SCHEMA_V2.to_owned(),
        batch_precommit_root_sha256: batch.batch_root_sha256.clone(),
        case_preverification_root_sha256: case.receipt_root_sha256.clone(),
        closure_plan: plan.clone(),
        items,
        workspace_denominator_root_sha256,
        all_requests_precommitted: true,
        authority: denied_authority_v1(),
        dispatch_root_sha256: String::new(),
    };
    dispatch.reseal()?;
    Ok(dispatch)
}
