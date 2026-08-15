use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V2, K2_UNCERTAINTY_FINAL_VERIFIER_REQUEST_SCHEMA_V2,
    K2UncertaintyBatchPrecommitV2, K2UncertaintyCaseJournalEventKindV2,
    K2UncertaintyCaseJournalStateV2, K2UncertaintyCasePreverificationV2,
    K2UncertaintyObservationVectorV2, K2UncertaintyPlanDispatchV2, K2UncertaintyPrivateCaseV1,
    K2UncertaintyProbeArtifactsV1, K2UncertaintyProbeRequestV1, denied_authority_v1,
    require_denied_authority_v1, uncertainty_root_v1,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyFinalVerifierRequestV2 {
    pub schema: String,
    pub verifier_executable_sha256: String,
    pub batch_precommit: K2UncertaintyBatchPrecommitV2,
    pub probe_request: K2UncertaintyProbeRequestV1,
    pub probe_artifacts: K2UncertaintyProbeArtifactsV1,
    pub case_preverification: K2UncertaintyCasePreverificationV2,
    pub private_case: K2UncertaintyPrivateCaseV1,
    pub dispatch: K2UncertaintyPlanDispatchV2,
    pub observation_vector: K2UncertaintyObservationVectorV2,
    pub case_journal_state: K2UncertaintyCaseJournalStateV2,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyFinalVerifierRequestV2 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        verifier_executable_sha256: String,
        batch_precommit: K2UncertaintyBatchPrecommitV2,
        probe_request: K2UncertaintyProbeRequestV1,
        probe_artifacts: K2UncertaintyProbeArtifactsV1,
        case_preverification: K2UncertaintyCasePreverificationV2,
        private_case: K2UncertaintyPrivateCaseV1,
        dispatch: K2UncertaintyPlanDispatchV2,
        observation_vector: K2UncertaintyObservationVectorV2,
        case_journal_state: K2UncertaintyCaseJournalStateV2,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_FINAL_VERIFIER_REQUEST_SCHEMA_V2.to_owned(),
            verifier_executable_sha256,
            batch_precommit,
            probe_request,
            probe_artifacts,
            case_preverification,
            private_case,
            dispatch,
            observation_vector,
            case_journal_state,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.verifier_executable_sha256)?;
        self.batch_precommit.validate()?;
        self.probe_request.validate()?;
        self.probe_artifacts.validate()?;
        self.case_preverification.validate()?;
        self.private_case.validate()?;
        self.dispatch.validate()?;
        self.observation_vector
            .validate_against_dispatch(&self.dispatch)?;
        self.case_journal_state.validate()?;
        let case_id = &self.probe_request.public_case.vocabulary.case_id_sha256;
        let entry = self
            .batch_precommit
            .cases
            .iter()
            .find(|entry| &entry.case_id_sha256 == case_id)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_final_v2_batch_case_missing",
            ))?;
        let plan = self.case_preverification.closure_plan.as_ref().ok_or(
            K2CompositionErrorV1::Invalid("self_formed_final_v2_closure_unavailable"),
        )?;
        let observation_events = self
            .case_journal_state
            .events
            .iter()
            .filter(|event| {
                event.kind == K2UncertaintyCaseJournalEventKindV2::ProbeObservationFrozen
            })
            .collect::<Vec<_>>();
        let journal_observations_match = observation_events.len()
            == self.observation_vector.executions.len()
            && observation_events
                .iter()
                .zip(&self.observation_vector.executions)
                .all(|(event, execution)| {
                    event.probe_ordinal == Some(execution.probe_ordinal)
                        && event.workspace_identity_root_sha256.as_deref()
                            == Some(execution.workspace_identity_root_sha256.as_str())
                        && event.payload_root_sha256 == execution.observation.receipt_root_sha256
                });
        let vector_event_matches = self.case_journal_state.events.last().is_some_and(|event| {
            event.kind == K2UncertaintyCaseJournalEventKindV2::ObservationVectorFrozen
                && event.payload_root_sha256 == self.observation_vector.vector_root_sha256
        });
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_FINAL_VERIFIER_REQUEST_SCHEMA_V2
            || !self.batch_precommit.dispatch_permitted
            || self.batch_precommit.experiment_id_sha256
                != self
                    .probe_request
                    .public_case
                    .vocabulary
                    .experiment_id_sha256
            || self.private_case.experiment_id_sha256 != self.batch_precommit.experiment_id_sha256
            || self.probe_artifacts.probe_request_root_sha256
                != self.probe_request.request_root_sha256
            || self.probe_artifacts.case_id_sha256 != *case_id
            || self
                .case_preverification
                .selection_preverification
                .case_id_sha256
                != *case_id
            || self.private_case.case_id_sha256 != *case_id
            || self.private_case.public_case_root_sha256
                != self.probe_request.public_case.public_case_root_sha256
            || entry.case_preverification_root_sha256
                != self.case_preverification.receipt_root_sha256
            || entry.closure_plan_root_sha256.as_deref() != Some(plan.plan_root_sha256.as_str())
            || self.dispatch.batch_precommit_root_sha256 != self.batch_precommit.batch_root_sha256
            || self.dispatch.case_preverification_root_sha256
                != self.case_preverification.receipt_root_sha256
            || self.dispatch.closure_plan != *plan
            || self.case_journal_state.dispatch != self.dispatch
            || !journal_observations_match
            || !vector_event_matches
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_verifier_request_v2_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.request_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&K2UncertaintyFinalVerifierRequestRootV2 {
            schema: K2_UNCERTAINTY_FINAL_VERIFIER_REQUEST_SCHEMA_V2,
            verifier_executable_sha256: &self.verifier_executable_sha256,
            batch_precommit_root_sha256: &self.batch_precommit.batch_root_sha256,
            probe_request_root_sha256: &self.probe_request.request_root_sha256,
            probe_artifacts_root_sha256: &self.probe_artifacts.artifacts_root_sha256,
            case_preverification_root_sha256: &self.case_preverification.receipt_root_sha256,
            private_case_root_sha256: &self.private_case.private_case_root_sha256,
            dispatch_root_sha256: &self.dispatch.dispatch_root_sha256,
            observation_vector_root_sha256: &self.observation_vector.vector_root_sha256,
            case_journal_root_sha256: &self.case_journal_state.journal_root_sha256,
            authority: &self.authority,
        })
    }
}

#[derive(Serialize)]
struct K2UncertaintyFinalVerifierRequestRootV2<'a> {
    schema: &'static str,
    verifier_executable_sha256: &'a str,
    batch_precommit_root_sha256: &'a str,
    probe_request_root_sha256: &'a str,
    probe_artifacts_root_sha256: &'a str,
    case_preverification_root_sha256: &'a str,
    private_case_root_sha256: &'a str,
    dispatch_root_sha256: &'a str,
    observation_vector_root_sha256: &'a str,
    case_journal_root_sha256: &'a str,
    authority: &'a K2CompositionAuthorityBoundaryV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCaseVerificationReceiptV2 {
    pub schema: String,
    pub verifier_executable_sha256: String,
    pub verifier_request_root_sha256: String,
    pub case_id_sha256: String,
    pub closure_plan_root_sha256: String,
    pub observation_vector_root_sha256: String,
    pub consistency_dispositions: u64,
    pub materialized_models: u64,
    pub semantic_signature_outcomes: u64,
    pub raw_probe_dispositions: u64,
    pub raw_predictions: u64,
    pub representative_count: u64,
    pub tournament_requests: u64,
    pub adapted_predictions: u64,
    pub completion_candidate_count: u64,
    pub joint_pairwise_comparisons: u64,
    pub selected_probe_executions: u64,
    pub safety_verified: u64,
    pub worker_observer_matches: u64,
    pub surviving_semantic_classes: u64,
    pub private_true_class_match: bool,
    pub ordered_outcomes_precommitted: bool,
    pub false_accepts: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyCaseVerificationReceiptV2 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.receipt_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.verifier_executable_sha256,
            &self.verifier_request_root_sha256,
            &self.case_id_sha256,
            &self.closure_plan_root_sha256,
            &self.observation_vector_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V2
            || self.consistency_dispositions != 336
            || self.materialized_models != 4
            || self.semantic_signature_outcomes != 7_168
            || self.raw_probe_dispositions != 1_792
            || self.raw_predictions != 7_168
            || self.representative_count < 8
            || self.tournament_requests == 0
            || !(1..=2).contains(&self.selected_probe_executions)
            || self.safety_verified != self.selected_probe_executions
            || self.worker_observer_matches != self.selected_probe_executions
            || self.surviving_semantic_classes != 1
            || !self.private_true_class_match
            || !self.ordered_outcomes_precommitted
            || self.false_accepts != 0
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_verification_receipt_v2_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&K2UncertaintyCaseVerificationReceiptRootV2 {
            schema: K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V2,
            verifier_executable_sha256: &self.verifier_executable_sha256,
            verifier_request_root_sha256: &self.verifier_request_root_sha256,
            case_id_sha256: &self.case_id_sha256,
            closure_plan_root_sha256: &self.closure_plan_root_sha256,
            observation_vector_root_sha256: &self.observation_vector_root_sha256,
            consistency_dispositions: self.consistency_dispositions,
            materialized_models: self.materialized_models,
            semantic_signature_outcomes: self.semantic_signature_outcomes,
            raw_probe_dispositions: self.raw_probe_dispositions,
            raw_predictions: self.raw_predictions,
            representative_count: self.representative_count,
            tournament_requests: self.tournament_requests,
            adapted_predictions: self.adapted_predictions,
            completion_candidate_count: self.completion_candidate_count,
            joint_pairwise_comparisons: self.joint_pairwise_comparisons,
            selected_probe_executions: self.selected_probe_executions,
            safety_verified: self.safety_verified,
            worker_observer_matches: self.worker_observer_matches,
            surviving_semantic_classes: self.surviving_semantic_classes,
            private_true_class_match: self.private_true_class_match,
            ordered_outcomes_precommitted: self.ordered_outcomes_precommitted,
            false_accepts: self.false_accepts,
            authority: &self.authority,
        })
    }
}

#[derive(Serialize)]
struct K2UncertaintyCaseVerificationReceiptRootV2<'a> {
    schema: &'static str,
    verifier_executable_sha256: &'a str,
    verifier_request_root_sha256: &'a str,
    case_id_sha256: &'a str,
    closure_plan_root_sha256: &'a str,
    observation_vector_root_sha256: &'a str,
    consistency_dispositions: u64,
    materialized_models: u64,
    semantic_signature_outcomes: u64,
    raw_probe_dispositions: u64,
    raw_predictions: u64,
    representative_count: u64,
    tournament_requests: u64,
    adapted_predictions: u64,
    completion_candidate_count: u64,
    joint_pairwise_comparisons: u64,
    selected_probe_executions: u64,
    safety_verified: u64,
    worker_observer_matches: u64,
    surviving_semantic_classes: u64,
    private_true_class_match: bool,
    ordered_outcomes_precommitted: bool,
    false_accepts: u64,
    authority: &'a K2CompositionAuthorityBoundaryV1,
}
