use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    K2InquiryObserverRequestV1, K2InquiryProbeV1, K2InquiryWorkerRequestV1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_PLAN_DISPATCH_SCHEMA_V2, K2_UNCERTAINTY_PROBE_DISPATCH_ITEM_SCHEMA_V2,
    K2_UNCERTAINTY_WORKSPACE_IDENTITY_SCHEMA_V2, K2UncertaintyClosurePlanV1,
    K2UncertaintyPrivateSafetyDispositionV1, K2UncertaintySafetyReceiptV1,
    K2UncertaintySafetyRequestV1, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyWorkspaceIdentityV2 {
    pub schema: String,
    pub case_id_sha256: String,
    pub closure_plan_root_sha256: String,
    pub probe_ordinal: u64,
    pub identity_root_sha256: String,
}

impl K2UncertaintyWorkspaceIdentityV2 {
    pub fn seal(
        case_id_sha256: String,
        closure_plan_root_sha256: String,
        probe_ordinal: u64,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_WORKSPACE_IDENTITY_SCHEMA_V2.to_owned(),
            case_id_sha256,
            closure_plan_root_sha256,
            probe_ordinal,
            identity_root_sha256: String::new(),
        };
        value.identity_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.case_id_sha256)?;
        require_composition_root_v1(&self.closure_plan_root_sha256)?;
        if self.schema != K2_UNCERTAINTY_WORKSPACE_IDENTITY_SCHEMA_V2
            || self.identity_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_workspace_identity_v2_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_WORKSPACE_IDENTITY_SCHEMA_V2,
            &self.case_id_sha256,
            &self.closure_plan_root_sha256,
            self.probe_ordinal,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyProbeDispatchItemV2 {
    pub schema: String,
    pub case_id_sha256: String,
    pub closure_plan_root_sha256: String,
    pub probe_ordinal: u64,
    pub selected_probe: K2InquiryProbeV1,
    pub safety_request: K2UncertaintySafetyRequestV1,
    pub safety_receipt: K2UncertaintySafetyReceiptV1,
    pub worker_request: K2InquiryWorkerRequestV1,
    pub observer_request: K2InquiryObserverRequestV1,
    pub workspace_identity: K2UncertaintyWorkspaceIdentityV2,
    pub initial_manifest_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub item_root_sha256: String,
}

impl K2UncertaintyProbeDispatchItemV2 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.item_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.closure_plan_root_sha256,
            &self.initial_manifest_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.selected_probe.validate()?;
        self.safety_request.validate()?;
        self.safety_receipt.validate()?;
        self.worker_request.validate()?;
        self.observer_request.validate()?;
        self.workspace_identity.validate()?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_PROBE_DISPATCH_ITEM_SCHEMA_V2
            || self.selected_probe.experiment_id_sha256 != self.case_id_sha256
            || self.selected_probe.probe_root_sha256
                != self.worker_request.selected_probe_root_sha256
            || self.selected_probe.probe_root_sha256
                != self.observer_request.selected_probe_root_sha256
            || self.selected_probe != self.safety_request.selected_probe
            || self.safety_request.selection_root_sha256 != self.closure_plan_root_sha256
            || self.safety_receipt.safety_request_root_sha256
                != self.safety_request.request_root_sha256
            || self.safety_receipt.disposition != K2UncertaintyPrivateSafetyDispositionV1::Pass
            || self.worker_request.selection_verification_root_sha256
                != self.closure_plan_root_sha256
            || self.worker_request.initial_manifest != self.selected_probe.initial_manifest
            || self.worker_request.resolved_effect != self.safety_request.resolved_private_effect
            || self.observer_request.experiment_id_sha256 != self.case_id_sha256
            || self.workspace_identity.case_id_sha256 != self.case_id_sha256
            || self.workspace_identity.closure_plan_root_sha256 != self.closure_plan_root_sha256
            || self.workspace_identity.probe_ordinal != self.probe_ordinal
            || self.initial_manifest_root_sha256
                != self.selected_probe.initial_manifest.tree_root_sha256
            || self.item_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_probe_dispatch_item_v2_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PROBE_DISPATCH_ITEM_SCHEMA_V2,
            &self.case_id_sha256,
            &self.closure_plan_root_sha256,
            self.probe_ordinal,
            &self.selected_probe.probe_root_sha256,
            &self.safety_request.request_root_sha256,
            &self.safety_receipt.receipt_root_sha256,
            &self.worker_request.request_root_sha256,
            &self.observer_request.request_root_sha256,
            &self.workspace_identity.identity_root_sha256,
            &self.initial_manifest_root_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyPlanDispatchV2 {
    pub schema: String,
    pub batch_precommit_root_sha256: String,
    pub case_preverification_root_sha256: String,
    pub closure_plan: K2UncertaintyClosurePlanV1,
    pub items: Vec<K2UncertaintyProbeDispatchItemV2>,
    pub workspace_denominator_root_sha256: String,
    pub all_requests_precommitted: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub dispatch_root_sha256: String,
}

impl K2UncertaintyPlanDispatchV2 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.dispatch_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.batch_precommit_root_sha256,
            &self.case_preverification_root_sha256,
            &self.workspace_denominator_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.closure_plan.validate()?;
        let mut workspaces = BTreeSet::new();
        for (ordinal, item) in self.items.iter().enumerate() {
            item.validate()?;
            if item.case_id_sha256 != self.closure_plan.case_id_sha256
                || item.closure_plan_root_sha256 != self.closure_plan.plan_root_sha256
                || item.probe_ordinal != ordinal as u64
                || self.closure_plan.ordered_probe_roots_sha256[ordinal]
                    != item.selected_probe.probe_root_sha256
                || !workspaces.insert(item.workspace_identity.identity_root_sha256.clone())
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_plan_dispatch_item_v2_invalid",
                ));
            }
        }
        let workspace_roots = self
            .items
            .iter()
            .map(|value| value.workspace_identity.identity_root_sha256.as_str())
            .collect::<Vec<_>>();
        let expected_workspace_denominator = uncertainty_root_v1(&(
            "nando.k2-self-formed-workspace-denominator.v2",
            &self.closure_plan.plan_root_sha256,
            workspace_roots,
        ))?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_PLAN_DISPATCH_SCHEMA_V2
            || !self.all_requests_precommitted
            || self.items.len() as u64 != self.closure_plan.plan_length
            || self.workspace_denominator_root_sha256 != expected_workspace_denominator
            || self.dispatch_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_plan_dispatch_v2_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        let item_roots = self
            .items
            .iter()
            .map(|value| value.item_root_sha256.as_str())
            .collect::<Vec<_>>();
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PLAN_DISPATCH_SCHEMA_V2,
            &self.batch_precommit_root_sha256,
            &self.case_preverification_root_sha256,
            &self.closure_plan.plan_root_sha256,
            item_roots,
            &self.workspace_denominator_root_sha256,
            self.all_requests_precommitted,
            &self.authority,
        ))
    }
}

pub(crate) fn blank_probe_dispatch_item_v2(
    case_id_sha256: String,
    closure_plan_root_sha256: String,
    probe_ordinal: u64,
    selected_probe: K2InquiryProbeV1,
    safety_request: K2UncertaintySafetyRequestV1,
    safety_receipt: K2UncertaintySafetyReceiptV1,
    worker_request: K2InquiryWorkerRequestV1,
    observer_request: K2InquiryObserverRequestV1,
    workspace_identity: K2UncertaintyWorkspaceIdentityV2,
) -> K2UncertaintyProbeDispatchItemV2 {
    K2UncertaintyProbeDispatchItemV2 {
        schema: K2_UNCERTAINTY_PROBE_DISPATCH_ITEM_SCHEMA_V2.to_owned(),
        case_id_sha256,
        closure_plan_root_sha256,
        probe_ordinal,
        initial_manifest_root_sha256: selected_probe.initial_manifest.tree_root_sha256.clone(),
        selected_probe,
        safety_request,
        safety_receipt,
        worker_request,
        observer_request,
        workspace_identity,
        authority: denied_authority_v1(),
        item_root_sha256: String::new(),
    }
}
