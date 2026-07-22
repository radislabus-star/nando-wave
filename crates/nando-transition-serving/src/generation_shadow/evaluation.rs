use std::sync::Arc;

use nando_operator_kernel::{RuntimePhaseControlEvidenceV3, canonical_json_sha256};
use nando_operator_proof::independent_verifier_v3::{
    IndependentVerifierBudgetV3, IndependentVerifierInputV3, IndependentVerifierReceiptV3,
    IndependentVerifierVerdictV3, verify_operator_result_v3,
};
use nando_operator_runtime::{
    RuntimeContextBudgetV3, TrafficShadowInputV3, TrafficShadowReceiptV3, TrafficShadowSourceV3,
    TrafficShadowVerdictV3, execute_traffic_shadow_with_handoff_v3,
};

pub(super) struct GenerationShadowEvaluationV3 {
    receipt: GenerationShadowEvaluationReceiptV3,
    traffic_receipt: Option<TrafficShadowReceiptV3>,
    verifier_receipt: Option<IndependentVerifierReceiptV3>,
    phase_control_evidence: Option<RuntimePhaseControlEvidenceV3>,
}

impl GenerationShadowEvaluationV3 {
    pub(super) const fn receipt(&self) -> &GenerationShadowEvaluationReceiptV3 {
        &self.receipt
    }

    pub(super) const fn traffic_receipt(&self) -> Option<&TrafficShadowReceiptV3> {
        self.traffic_receipt.as_ref()
    }

    pub(super) const fn verifier_receipt(&self) -> Option<&IndependentVerifierReceiptV3> {
        self.verifier_receipt.as_ref()
    }

    pub(super) const fn phase_control_evidence(&self) -> Option<&RuntimePhaseControlEvidenceV3> {
        self.phase_control_evidence.as_ref()
    }

    pub(super) fn into_receipt(self) -> GenerationShadowEvaluationReceiptV3 {
        self.receipt
    }
}

use super::{
    GenerationShadowEvaluationReceiptV3, GenerationShadowEvaluationVerdictV3,
    GenerationShadowRequestV3, GenerationShadowSnapshotV3,
};

#[must_use]
pub fn evaluate_generation_shadow_request_v3(
    generation: &GenerationShadowSnapshotV3,
    request: &GenerationShadowRequestV3,
) -> GenerationShadowEvaluationReceiptV3 {
    evaluate_generation_shadow_request_with_evidence_v3(generation, request).into_receipt()
}

pub(super) fn evaluate_generation_shadow_request_with_evidence_v3(
    generation: &GenerationShadowSnapshotV3,
    request: &GenerationShadowRequestV3,
) -> GenerationShadowEvaluationV3 {
    let payload = match serde_json::from_slice(request.provider_payload_bytes()) {
        Ok(payload) => payload,
        Err(_) => return invalid_request_evaluation(generation, request),
    };
    let input = match TrafficShadowInputV3::replayable(
        request.window_row_sha256(),
        request.request_sha256(),
        request.projection(),
        request.streaming(),
        TrafficShadowSourceV3::Ordinary,
        request.request_text(),
        &payload,
    ) {
        Ok(input) => input,
        Err(_) => return invalid_request_evaluation(generation, request),
    };
    let execution = execute_traffic_shadow_with_handoff_v3(
        Arc::clone(generation.traffic_generation()),
        input,
        RuntimeContextBudgetV3::default(),
    );
    let traffic_receipt = execution.receipt();
    let phase_control_evidence = execution.phase_control_evidence().cloned();
    let parity_mismatch =
        traffic_receipt.verdict() == TrafficShadowVerdictV3::ActorVmParityMismatch;
    if traffic_receipt.verdict() != TrafficShadowVerdictV3::CompleteShadow {
        return evaluation(
            generation,
            request,
            Some(traffic_receipt.clone()),
            None,
            phase_control_evidence,
            if parity_mismatch || is_runtime_reject(traffic_receipt.verdict()) {
                GenerationShadowEvaluationVerdictV3::RuntimeReject
            } else {
                GenerationShadowEvaluationVerdictV3::RuntimeAbstain
            },
            parity_mismatch,
        );
    }
    let (Some(action), Some(output)) = (execution.actor_action(), execution.actor_output()) else {
        return evaluation(
            generation,
            request,
            Some(traffic_receipt.clone()),
            None,
            phase_control_evidence,
            GenerationShadowEvaluationVerdictV3::RuntimeReject,
            false,
        );
    };
    let verifier_input = match IndependentVerifierInputV3::new(
        request.request_sha256(),
        request.projection(),
        request.provider_payload_bytes(),
        generation.verifier_artifacts(),
        action,
        output,
    ) {
        Ok(input) => input,
        Err(_) => {
            return evaluation(
                generation,
                request,
                Some(traffic_receipt.clone()),
                None,
                phase_control_evidence,
                GenerationShadowEvaluationVerdictV3::VerifierReject,
                false,
            );
        }
    };
    match verify_operator_result_v3(&verifier_input, IndependentVerifierBudgetV3::default()) {
        Ok(verifier) => {
            let verdict = verifier_verdict(verifier.verdict());
            evaluation(
                generation,
                request,
                Some(traffic_receipt.clone()),
                Some(verifier),
                phase_control_evidence,
                verdict,
                false,
            )
        }
        Err(_) => evaluation(
            generation,
            request,
            Some(traffic_receipt.clone()),
            None,
            phase_control_evidence,
            GenerationShadowEvaluationVerdictV3::VerifierReject,
            false,
        ),
    }
}

fn invalid_request_evaluation(
    generation: &GenerationShadowSnapshotV3,
    request: &GenerationShadowRequestV3,
) -> GenerationShadowEvaluationV3 {
    let traffic_receipt_sha256 = canonical_json_sha256(&(
        "nando.generation-shadow-invalid-request.v3.f7",
        generation
            .checkpoint()
            .generation()
            .manifest()
            .generation_id_sha256(),
        request.request_sha256(),
    ))
    .unwrap_or_else(|_| "0".repeat(64));
    GenerationShadowEvaluationV3 {
        receipt: receipt(
            generation,
            request,
            traffic_receipt_sha256,
            None,
            GenerationShadowEvaluationVerdictV3::InvalidRequest,
            false,
        ),
        traffic_receipt: None,
        verifier_receipt: None,
        phase_control_evidence: None,
    }
}

fn evaluation(
    generation: &GenerationShadowSnapshotV3,
    request: &GenerationShadowRequestV3,
    traffic_receipt: Option<TrafficShadowReceiptV3>,
    verifier_receipt: Option<IndependentVerifierReceiptV3>,
    phase_control_evidence: Option<RuntimePhaseControlEvidenceV3>,
    verdict: GenerationShadowEvaluationVerdictV3,
    parity_mismatch: bool,
) -> GenerationShadowEvaluationV3 {
    let traffic_receipt_sha256 = traffic_receipt
        .as_ref()
        .map(|receipt| receipt.receipt_sha256().to_owned())
        .unwrap_or_else(|| "0".repeat(64));
    let verifier_receipt_sha256 = verifier_receipt
        .as_ref()
        .map(|receipt| receipt.receipt_sha256().to_owned());
    GenerationShadowEvaluationV3 {
        receipt: receipt(
            generation,
            request,
            traffic_receipt_sha256,
            verifier_receipt_sha256,
            verdict,
            parity_mismatch,
        ),
        traffic_receipt,
        verifier_receipt,
        phase_control_evidence,
    }
}

fn receipt(
    generation: &GenerationShadowSnapshotV3,
    request: &GenerationShadowRequestV3,
    traffic_receipt_sha256: String,
    verifier_receipt_sha256: Option<String>,
    verdict: GenerationShadowEvaluationVerdictV3,
    parity_mismatch: bool,
) -> GenerationShadowEvaluationReceiptV3 {
    GenerationShadowEvaluationReceiptV3 {
        generation_id_sha256: generation
            .checkpoint()
            .generation()
            .manifest()
            .generation_id_sha256()
            .to_owned(),
        publish_sequence: generation.checkpoint().publish_sequence(),
        request_sha256: request.request_sha256().to_owned(),
        capture_sequence: request
            .capture_receipt()
            .map(|receipt| receipt.capture_sequence()),
        capture_event_sha256: request
            .capture_receipt()
            .map(|receipt| receipt.event_root_sha256().to_hex()),
        capture_receipt_sha256: request
            .capture_receipt()
            .map(|receipt| receipt.receipt_sha256().to_hex()),
        traffic_receipt_sha256,
        verifier_receipt_sha256,
        verdict,
        parity_mismatch,
        raw_payloads_persisted: 0,
        local_accepts: 0,
        execution_authority: false,
    }
}

const fn is_runtime_reject(verdict: TrafficShadowVerdictV3) -> bool {
    matches!(
        verdict,
        TrafficShadowVerdictV3::RejectInvariantMismatch
            | TrafficShadowVerdictV3::ActorVmParityMismatch
    )
}

const fn verifier_verdict(
    verdict: IndependentVerifierVerdictV3,
) -> GenerationShadowEvaluationVerdictV3 {
    match verdict {
        IndependentVerifierVerdictV3::Verified => GenerationShadowEvaluationVerdictV3::Verified,
        IndependentVerifierVerdictV3::AbstainUnsupportedProjection
        | IndependentVerifierVerdictV3::AbstainBudgetExhausted
        | IndependentVerifierVerdictV3::AbstainMissingRole
        | IndependentVerifierVerdictV3::AbstainMissingCapability
        | IndependentVerifierVerdictV3::AbstainAmbiguousCandidate
        | IndependentVerifierVerdictV3::AbstainUnsupportedEffect => {
            GenerationShadowEvaluationVerdictV3::VerifierAbstain
        }
        _ => GenerationShadowEvaluationVerdictV3::VerifierReject,
    }
}
