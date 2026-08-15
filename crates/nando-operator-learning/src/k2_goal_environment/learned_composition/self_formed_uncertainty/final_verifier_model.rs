use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    K2InquiryObservationReceiptV1, K2InquiryObserverRequestV1, K2InquiryWorkerOutcomeV1,
    K2InquiryWorkerRequestV1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V1, K2_UNCERTAINTY_FINAL_VERIFIER_REQUEST_SCHEMA_V1,
    K2UncertaintyCasePreverificationV1, K2UncertaintyDispatchReceiptV1, K2UncertaintyPrivateCaseV1,
    K2UncertaintyProbeArtifactsV1, K2UncertaintyProbeRequestV1, K2UncertaintySafetyReceiptV1,
    K2UncertaintySafetyRequestV1, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyFinalVerifierRequestV1 {
    pub schema: String,
    pub verifier_executable_sha256: String,
    pub probe_request: K2UncertaintyProbeRequestV1,
    pub probe_artifacts: K2UncertaintyProbeArtifactsV1,
    pub case_preverification: K2UncertaintyCasePreverificationV1,
    pub private_case: K2UncertaintyPrivateCaseV1,
    pub safety_request: K2UncertaintySafetyRequestV1,
    pub safety_receipt: K2UncertaintySafetyReceiptV1,
    pub dispatch_receipt: K2UncertaintyDispatchReceiptV1,
    pub worker_request: K2InquiryWorkerRequestV1,
    pub observer_request: K2InquiryObserverRequestV1,
    pub worker_outcome: K2InquiryWorkerOutcomeV1,
    pub observation: K2InquiryObservationReceiptV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyFinalVerifierRequestV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        verifier_executable_sha256: String,
        probe_request: K2UncertaintyProbeRequestV1,
        probe_artifacts: K2UncertaintyProbeArtifactsV1,
        case_preverification: K2UncertaintyCasePreverificationV1,
        private_case: K2UncertaintyPrivateCaseV1,
        safety_request: K2UncertaintySafetyRequestV1,
        safety_receipt: K2UncertaintySafetyReceiptV1,
        dispatch_receipt: K2UncertaintyDispatchReceiptV1,
        worker_request: K2InquiryWorkerRequestV1,
        observer_request: K2InquiryObserverRequestV1,
        worker_outcome: K2InquiryWorkerOutcomeV1,
        observation: K2InquiryObservationReceiptV1,
    ) -> K2CompositionResultV1<Self> {
        let authority = denied_authority_v1();
        let mut value = Self {
            schema: K2_UNCERTAINTY_FINAL_VERIFIER_REQUEST_SCHEMA_V1.to_owned(),
            verifier_executable_sha256,
            probe_request,
            probe_artifacts,
            case_preverification,
            private_case,
            safety_request,
            safety_receipt,
            dispatch_receipt,
            worker_request,
            observer_request,
            worker_outcome,
            observation,
            authority,
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.verifier_executable_sha256)?;
        self.probe_request.validate()?;
        self.probe_artifacts.validate()?;
        self.case_preverification.validate()?;
        self.private_case.validate()?;
        self.safety_request.validate()?;
        self.safety_receipt.validate()?;
        self.dispatch_receipt.validate()?;
        self.worker_request.validate()?;
        self.observer_request.validate()?;
        let mut worker = self.worker_outcome.clone();
        worker.reseal()?;
        let mut observation = self.observation.clone();
        observation.reseal()?;
        require_denied_authority_v1(&self.worker_outcome.authority)?;
        require_denied_authority_v1(&self.observation.authority)?;
        require_denied_authority_v1(&self.authority)?;
        let case_id = &self.probe_request.public_case.vocabulary.case_id_sha256;
        if self.schema != K2_UNCERTAINTY_FINAL_VERIFIER_REQUEST_SCHEMA_V1
            || self.probe_artifacts.probe_request_root_sha256
                != self.probe_request.request_root_sha256
            || self.probe_artifacts.case_id_sha256 != *case_id
            || self.case_preverification.case_id_sha256 != *case_id
            || self.private_case.case_id_sha256 != *case_id
            || self.private_case.public_case_root_sha256
                != self.probe_request.public_case.public_case_root_sha256
            || self.safety_request.selection_root_sha256
                != self.case_preverification.receipt_root_sha256
            || self.safety_receipt.safety_request_root_sha256
                != self.safety_request.request_root_sha256
            || self.dispatch_receipt.case_preverification_root_sha256
                != self.case_preverification.receipt_root_sha256
            || self.dispatch_receipt.safety_receipt_root_sha256
                != self.safety_receipt.receipt_root_sha256
            || self.dispatch_receipt.worker_request_root_sha256
                != self.worker_request.request_root_sha256
            || self.dispatch_receipt.observer_request_root_sha256
                != self.observer_request.request_root_sha256
            || self.worker_outcome.request_root_sha256 != self.worker_request.request_root_sha256
            || self.observation.observer_request_root_sha256
                != self.observer_request.request_root_sha256
            || worker != self.worker_outcome
            || observation != self.observation
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_verifier_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_FINAL_VERIFIER_REQUEST_SCHEMA_V1,
            &self.verifier_executable_sha256,
            &self.probe_request,
            &self.probe_artifacts,
            &self.case_preverification,
            &self.private_case,
            &self.safety_request,
            &self.safety_receipt,
            &self.dispatch_receipt,
            &self.worker_request,
            &self.observer_request,
            &self.worker_outcome,
            &self.observation,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCaseVerificationReceiptV1 {
    pub schema: String,
    pub verifier_executable_sha256: String,
    pub verifier_request_root_sha256: String,
    pub case_id_sha256: String,
    pub consistency_dispositions: u64,
    pub materialized_models: u64,
    pub semantic_signature_outcomes: u64,
    pub raw_probe_dispositions: u64,
    pub raw_predictions: u64,
    pub representative_count: u64,
    pub tournament_requests: u64,
    pub adapted_predictions: u64,
    pub safety_verified: bool,
    pub worker_observer_match: bool,
    pub surviving_semantic_classes: u64,
    pub private_true_class_match: bool,
    pub selected_outcome_precommitted: bool,
    pub false_accepts: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

#[derive(Serialize)]
struct K2UncertaintyCaseVerificationReceiptPayloadV1<'a> {
    schema: &'static str,
    verifier_executable_sha256: &'a str,
    verifier_request_root_sha256: &'a str,
    case_id_sha256: &'a str,
    consistency_dispositions: u64,
    materialized_models: u64,
    semantic_signature_outcomes: u64,
    raw_probe_dispositions: u64,
    raw_predictions: u64,
    representative_count: u64,
    tournament_requests: u64,
    adapted_predictions: u64,
    safety_verified: bool,
    worker_observer_match: bool,
    surviving_semantic_classes: u64,
    private_true_class_match: bool,
    selected_outcome_precommitted: bool,
    false_accepts: u64,
    authority: &'a K2CompositionAuthorityBoundaryV1,
}

impl K2UncertaintyCaseVerificationReceiptV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.receipt_root_sha256 = self.expected_root()?;
        self.validate()
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.verifier_executable_sha256,
            &self.verifier_request_root_sha256,
            &self.case_id_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = self.expected_root()?;
        if self.schema != K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V1
            || self.consistency_dispositions != 336
            || self.materialized_models != 4
            || self.semantic_signature_outcomes != 7_168
            || self.raw_probe_dispositions != 1_792
            || self.raw_predictions != 7_168
            || self.representative_count < 8
            || self.tournament_requests == 0
            || !self.safety_verified
            || !self.worker_observer_match
            || self.surviving_semantic_classes != 1
            || !self.private_true_class_match
            || !self.selected_outcome_precommitted
            || self.false_accepts != 0
            || self.receipt_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_verification_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&K2UncertaintyCaseVerificationReceiptPayloadV1 {
            schema: K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V1,
            verifier_executable_sha256: &self.verifier_executable_sha256,
            verifier_request_root_sha256: &self.verifier_request_root_sha256,
            case_id_sha256: &self.case_id_sha256,
            consistency_dispositions: self.consistency_dispositions,
            materialized_models: self.materialized_models,
            semantic_signature_outcomes: self.semantic_signature_outcomes,
            raw_probe_dispositions: self.raw_probe_dispositions,
            raw_predictions: self.raw_predictions,
            representative_count: self.representative_count,
            tournament_requests: self.tournament_requests,
            adapted_predictions: self.adapted_predictions,
            safety_verified: self.safety_verified,
            worker_observer_match: self.worker_observer_match,
            surviving_semantic_classes: self.surviving_semantic_classes,
            private_true_class_match: self.private_true_class_match,
            selected_outcome_precommitted: self.selected_outcome_precommitted,
            false_accepts: self.false_accepts,
            authority: &self.authority,
        })
    }
}
