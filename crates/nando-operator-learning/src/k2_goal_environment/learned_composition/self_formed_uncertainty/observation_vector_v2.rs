use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    K2InquiryObservationModeV1, K2InquiryObservationReceiptV1, K2InquiryWorkerOutcomeV1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_MAX_PLAN_PROBES_V1, K2_UNCERTAINTY_OBSERVATION_VECTOR_SCHEMA_V2,
    K2_UNCERTAINTY_PROBE_EXECUTION_EVIDENCE_SCHEMA_V2, K2UncertaintyPlanDispatchV2,
    K2UncertaintyProbeDispatchItemV2, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyProbeExecutionEvidenceV2 {
    pub schema: String,
    pub case_id_sha256: String,
    pub closure_plan_root_sha256: String,
    pub dispatch_root_sha256: String,
    pub probe_ordinal: u64,
    pub selected_probe_root_sha256: String,
    pub initial_manifest_root_sha256: String,
    pub workspace_identity_root_sha256: String,
    pub dispatch_item_root_sha256: String,
    pub worker_request_root_sha256: String,
    pub observer_request_root_sha256: String,
    pub worker_outcome: K2InquiryWorkerOutcomeV1,
    pub observation: K2InquiryObservationReceiptV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub evidence_root_sha256: String,
}

impl K2UncertaintyProbeExecutionEvidenceV2 {
    pub fn seal(
        dispatch_root_sha256: String,
        item: &K2UncertaintyProbeDispatchItemV2,
        worker_outcome: K2InquiryWorkerOutcomeV1,
        observation: K2InquiryObservationReceiptV1,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_PROBE_EXECUTION_EVIDENCE_SCHEMA_V2.to_owned(),
            case_id_sha256: item.case_id_sha256.clone(),
            closure_plan_root_sha256: item.closure_plan_root_sha256.clone(),
            dispatch_root_sha256,
            probe_ordinal: item.probe_ordinal,
            selected_probe_root_sha256: item.selected_probe.probe_root_sha256.clone(),
            initial_manifest_root_sha256: item.initial_manifest_root_sha256.clone(),
            workspace_identity_root_sha256: item.workspace_identity.identity_root_sha256.clone(),
            dispatch_item_root_sha256: item.item_root_sha256.clone(),
            worker_request_root_sha256: item.worker_request.request_root_sha256.clone(),
            observer_request_root_sha256: item.observer_request.request_root_sha256.clone(),
            worker_outcome,
            observation,
            authority: denied_authority_v1(),
            evidence_root_sha256: String::new(),
        };
        value.evidence_root_sha256 = value.expected_root()?;
        value.validate_against_item(item)?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.closure_plan_root_sha256,
            &self.dispatch_root_sha256,
            &self.selected_probe_root_sha256,
            &self.initial_manifest_root_sha256,
            &self.workspace_identity_root_sha256,
            &self.dispatch_item_root_sha256,
            &self.worker_request_root_sha256,
            &self.observer_request_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        let mut worker = self.worker_outcome.clone();
        worker.reseal()?;
        let mut observation = self.observation.clone();
        observation.reseal()?;
        require_denied_authority_v1(&self.worker_outcome.authority)?;
        require_denied_authority_v1(&self.observation.authority)?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_PROBE_EXECUTION_EVIDENCE_SCHEMA_V2
            || self.worker_outcome.request_root_sha256 != self.worker_request_root_sha256
            || self.observation.observer_request_root_sha256 != self.observer_request_root_sha256
            || self.worker_outcome.selected_probe_root_sha256 != self.selected_probe_root_sha256
            || self.observation.selected_probe_root_sha256 != self.selected_probe_root_sha256
            || self.worker_outcome.pre_manifest.tree_root_sha256
                != self.initial_manifest_root_sha256
            || self.worker_outcome.post_manifest != self.observation.post_manifest
            || worker != self.worker_outcome
            || observation != self.observation
            || self.evidence_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_probe_execution_evidence_v2_invalid",
            ));
        }
        Ok(())
    }

    pub fn validate_against_item(
        &self,
        item: &K2UncertaintyProbeDispatchItemV2,
    ) -> K2CompositionResultV1<()> {
        self.validate()?;
        item.validate()?;
        if self.case_id_sha256 != item.case_id_sha256
            || self.closure_plan_root_sha256 != item.closure_plan_root_sha256
            || self.probe_ordinal != item.probe_ordinal
            || self.selected_probe_root_sha256 != item.selected_probe.probe_root_sha256
            || self.initial_manifest_root_sha256 != item.initial_manifest_root_sha256
            || self.workspace_identity_root_sha256 != item.workspace_identity.identity_root_sha256
            || self.dispatch_item_root_sha256 != item.item_root_sha256
            || self.worker_request_root_sha256 != item.worker_request.request_root_sha256
            || self.observer_request_root_sha256 != item.observer_request.request_root_sha256
            || self.worker_outcome.worker_executable_sha256
                != item.worker_request.worker_executable_sha256
            || self.observation.observer_executable_sha256
                != item.observer_request.observer_executable_sha256
            || item.selected_probe.observation_mode != K2InquiryObservationModeV1::ExactImmediate
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_probe_execution_dispatch_binding_v2_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.evidence_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_PROBE_EXECUTION_EVIDENCE_SCHEMA_V2,
            &self.case_id_sha256,
            &self.closure_plan_root_sha256,
            &self.dispatch_root_sha256,
            self.probe_ordinal,
            &self.selected_probe_root_sha256,
            &self.initial_manifest_root_sha256,
            &self.workspace_identity_root_sha256,
            &self.dispatch_item_root_sha256,
            &self.worker_request_root_sha256,
            &self.observer_request_root_sha256,
            &self.worker_outcome,
            &self.observation,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyObservationVectorV2 {
    pub schema: String,
    pub case_id_sha256: String,
    pub closure_plan_root_sha256: String,
    pub dispatch_root_sha256: String,
    pub execution_evidence_roots_sha256: Vec<String>,
    pub ordered_observable_outcome_roots_sha256: Vec<String>,
    pub executions: Vec<K2UncertaintyProbeExecutionEvidenceV2>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub vector_root_sha256: String,
}

impl K2UncertaintyObservationVectorV2 {
    pub fn seal(
        dispatch: &K2UncertaintyPlanDispatchV2,
        executions: Vec<K2UncertaintyProbeExecutionEvidenceV2>,
    ) -> K2CompositionResultV1<Self> {
        dispatch.validate()?;
        let mut value = Self {
            schema: K2_UNCERTAINTY_OBSERVATION_VECTOR_SCHEMA_V2.to_owned(),
            case_id_sha256: dispatch.closure_plan.case_id_sha256.clone(),
            closure_plan_root_sha256: dispatch.closure_plan.plan_root_sha256.clone(),
            dispatch_root_sha256: dispatch.dispatch_root_sha256.clone(),
            execution_evidence_roots_sha256: executions
                .iter()
                .map(|entry| entry.evidence_root_sha256.clone())
                .collect(),
            ordered_observable_outcome_roots_sha256: executions
                .iter()
                .map(|entry| entry.observation.observable_outcome_root_sha256.clone())
                .collect(),
            executions,
            authority: denied_authority_v1(),
            vector_root_sha256: String::new(),
        };
        value.vector_root_sha256 = value.expected_root()?;
        value.validate_against_dispatch(dispatch)?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.closure_plan_root_sha256,
            &self.dispatch_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if self.executions.is_empty() || self.executions.len() > K2_UNCERTAINTY_MAX_PLAN_PROBES_V1 {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_observation_vector_length_v2_invalid",
            ));
        }
        let mut workspaces = BTreeSet::new();
        for (ordinal, execution) in self.executions.iter().enumerate() {
            execution.validate()?;
            if execution.case_id_sha256 != self.case_id_sha256
                || execution.closure_plan_root_sha256 != self.closure_plan_root_sha256
                || execution.dispatch_root_sha256 != self.dispatch_root_sha256
                || execution.probe_ordinal != ordinal as u64
                || !workspaces.insert(execution.workspace_identity_root_sha256.clone())
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_observation_vector_member_v2_invalid",
                ));
            }
        }
        let evidence_roots = self
            .executions
            .iter()
            .map(|entry| entry.evidence_root_sha256.clone())
            .collect::<Vec<_>>();
        let outcome_roots = self
            .executions
            .iter()
            .map(|entry| entry.observation.observable_outcome_root_sha256.clone())
            .collect::<Vec<_>>();
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_OBSERVATION_VECTOR_SCHEMA_V2
            || self.execution_evidence_roots_sha256 != evidence_roots
            || self.ordered_observable_outcome_roots_sha256 != outcome_roots
            || self.vector_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_observation_vector_v2_invalid",
            ));
        }
        Ok(())
    }

    pub fn validate_against_dispatch(
        &self,
        dispatch: &K2UncertaintyPlanDispatchV2,
    ) -> K2CompositionResultV1<()> {
        self.validate()?;
        dispatch.validate()?;
        if self.case_id_sha256 != dispatch.closure_plan.case_id_sha256
            || self.closure_plan_root_sha256 != dispatch.closure_plan.plan_root_sha256
            || self.dispatch_root_sha256 != dispatch.dispatch_root_sha256
            || self.executions.len() != dispatch.items.len()
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_observation_vector_dispatch_v2_invalid",
            ));
        }
        for (execution, item) in self.executions.iter().zip(&dispatch.items) {
            execution.validate_against_item(item)?;
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.execution_evidence_roots_sha256 = self
            .executions
            .iter()
            .map(|entry| entry.evidence_root_sha256.clone())
            .collect();
        self.ordered_observable_outcome_roots_sha256 = self
            .executions
            .iter()
            .map(|entry| entry.observation.observable_outcome_root_sha256.clone())
            .collect();
        self.vector_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_OBSERVATION_VECTOR_SCHEMA_V2,
            &self.case_id_sha256,
            &self.closure_plan_root_sha256,
            &self.dispatch_root_sha256,
            &self.execution_evidence_roots_sha256,
            &self.ordered_observable_outcome_roots_sha256,
            &self.authority,
        ))
    }
}
