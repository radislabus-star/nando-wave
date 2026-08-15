use std::collections::BTreeMap;

use super::super::{
    K2CompositionErrorV1, K2CompositionLearnedEffectV1, K2CompositionResultV1,
    K2InquiryBaselineDecisionV1, K2InquiryObserverRequestV1, K2InquiryProbeV1,
    K2InquiryVerifierCommandV1, K2InquiryVerifierReceiptV1, K2InquiryWorkerRequestV1,
};
use super::{
    K2_UNCERTAINTY_BASELINE_SOURCE_SHA256_V1, K2_UNCERTAINTY_BASELINE_SUMMARY_SCHEMA_V1,
    K2_UNCERTAINTY_CASE_PREVERIFICATION_SCHEMA_V1, K2_UNCERTAINTY_DISPATCH_RECEIPT_SCHEMA_V1,
    K2UncertaintyBaselineSummaryV1, K2UncertaintyBatchJournalProjectionV1,
    K2UncertaintyBatchPrecommitV1, K2UncertaintyCasePreverificationV1,
    K2UncertaintyDispatchReceiptV1, K2UncertaintyPrivateCaseV1,
    K2UncertaintyPrivateSafetyDispositionV1, K2UncertaintyProbeArtifactsV1,
    K2UncertaintyPublicCaseV1, K2UncertaintySafetyReceiptV1, K2UncertaintySafetyRequestV1,
    K2UncertaintyTournamentArtifactsV1, denied_authority_v1, uncertainty_root_v1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyDispatchArtifactsV1 {
    pub resolved_effect: K2CompositionLearnedEffectV1,
    pub worker_request: K2InquiryWorkerRequestV1,
    pub observer_request: K2InquiryObserverRequestV1,
    pub receipt: K2UncertaintyDispatchReceiptV1,
}

pub fn preverify_self_formed_case_with_owner_v1<V>(
    artifacts: &K2UncertaintyTournamentArtifactsV1,
    probe_artifacts: &K2UncertaintyProbeArtifactsV1,
    baseline_executable_sha256: &str,
    selection_verifier_executable_sha256: &str,
    verifier_owner: &mut V,
) -> K2CompositionResultV1<K2UncertaintyCasePreverificationV1>
where
    V: FnMut(&K2InquiryVerifierCommandV1) -> K2CompositionResultV1<K2InquiryVerifierReceiptV1>,
{
    artifacts.tournament.validate()?;
    probe_artifacts.validate()?;
    if probe_artifacts.case_id_sha256 != artifacts.tournament.case_id_sha256
        || probe_artifacts.frontier_root_sha256 != artifacts.tournament.frontier_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_preverification_artifact_binding_invalid",
        ));
    }
    let mut step_verifications = Vec::with_capacity(artifacts.steps.len());
    for (sequence, step) in artifacts.steps.iter().enumerate() {
        step.validate()?;
        if step.step_sequence != sequence as u64
            || step.frontier_root_sha256 != artifacts.tournament.frontier_root_sha256
            || artifacts.tournament.step_roots_sha256[sequence] != step.step_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_preverification_step_binding_invalid",
            ));
        }
        let command = K2InquiryVerifierCommandV1::VerifySelection {
            verifier_executable_sha256: selection_verifier_executable_sha256.to_owned(),
            selector_request: Box::new(step.request.clone()),
            precommit: Box::new(step.precommit.clone()),
        };
        let receipt = match verifier_owner(&command)? {
            K2InquiryVerifierReceiptV1::Selection { value } => value,
            K2InquiryVerifierReceiptV1::Outcome { .. } => {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_preverification_wrong_receipt",
                ));
            }
        };
        if receipt.public_case_root_sha256 != step.request.public_case.case_root_sha256
            || receipt.precommit_root_sha256 != step.precommit.precommit_root_sha256
            || receipt.selected_probe_root_sha256 != step.retained_probe_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_preverification_receipt_binding_invalid",
            ));
        }
        step_verifications.push(receipt);
    }
    let baseline_summary = summarize_baselines_v1(
        artifacts,
        baseline_executable_sha256,
        K2_UNCERTAINTY_BASELINE_SOURCE_SHA256_V1,
    )?;
    let mut receipt = K2UncertaintyCasePreverificationV1 {
        schema: K2_UNCERTAINTY_CASE_PREVERIFICATION_SCHEMA_V1.to_owned(),
        case_id_sha256: artifacts.tournament.case_id_sha256.clone(),
        probe_artifacts_root_sha256: probe_artifacts.artifacts_root_sha256.clone(),
        tournament: artifacts.tournament.clone(),
        selection_verifier_executable_sha256: selection_verifier_executable_sha256.to_owned(),
        step_verifications,
        baseline_summary,
        raw_probe_count: super::K2_UNCERTAINTY_RAW_PROBES_V1 as u64,
        raw_prediction_count: super::K2_UNCERTAINTY_RAW_PREDICTIONS_V1 as u64,
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    Ok(receipt)
}

fn summarize_baselines_v1(
    artifacts: &K2UncertaintyTournamentArtifactsV1,
    baseline_executable_sha256: &str,
    baseline_source_sha256: &str,
) -> K2CompositionResultV1<K2UncertaintyBaselineSummaryV1> {
    let mut decisions = Vec::with_capacity(artifacts.baselines.len());
    for trace in &artifacts.baselines {
        if trace.requests.len() != trace.outcomes.len()
            || trace
                .requests
                .iter()
                .any(|request| request.baseline_executable_sha256 != baseline_executable_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_baseline_trace_binding_invalid",
            ));
        }
        let decision_root_sha256 = uncertainty_root_v1(&(
            "nando.k2-inquiry-baseline-decision.v1",
            trace.kind,
            &trace.selected_probe_root_sha256,
        ))?;
        decisions.push(K2InquiryBaselineDecisionV1 {
            kind: trace.kind,
            selected_probe_root_sha256: trace.selected_probe_root_sha256.clone(),
            decision_root_sha256,
        });
    }
    let mut summary = K2UncertaintyBaselineSummaryV1 {
        schema: K2_UNCERTAINTY_BASELINE_SUMMARY_SCHEMA_V1.to_owned(),
        case_id_sha256: artifacts.tournament.case_id_sha256.clone(),
        frontier_root_sha256: artifacts.tournament.frontier_root_sha256.clone(),
        baseline_source_sha256: baseline_source_sha256.to_owned(),
        baseline_executable_sha256: baseline_executable_sha256.to_owned(),
        decisions,
        authority: denied_authority_v1(),
        summary_root_sha256: String::new(),
    };
    summary.reseal()?;
    Ok(summary)
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_self_formed_dispatch_v1(
    batch: &K2UncertaintyBatchPrecommitV1,
    case_preverification: &K2UncertaintyCasePreverificationV1,
    journal_projection: &K2UncertaintyBatchJournalProjectionV1,
    public_case: &K2UncertaintyPublicCaseV1,
    private_case: &K2UncertaintyPrivateCaseV1,
    selected_probe: &K2InquiryProbeV1,
    safety_request: &K2UncertaintySafetyRequestV1,
    safety_receipt: &K2UncertaintySafetyReceiptV1,
    worker_executable_sha256: &str,
    observer_executable_sha256: &str,
) -> K2CompositionResultV1<K2UncertaintyDispatchArtifactsV1> {
    batch.validate()?;
    case_preverification.validate()?;
    public_case.validate()?;
    private_case.validate()?;
    selected_probe.validate()?;
    safety_request.validate()?;
    safety_receipt.validate()?;
    if !journal_projection.all_cases_precommitted
        || journal_projection.experiment_id_sha256 != batch.experiment_id_sha256
        || journal_projection.execution_order_case_roots_sha256
            != batch.execution_order_case_roots_sha256
        || journal_projection
            .all_cases_precommitted_payload_root_sha256
            .as_deref()
            != Some(batch.batch_root_sha256.as_str())
        || journal_projection
            .indeterminate_dispatch_case_id_sha256
            .is_some()
        || !batch.cases.iter().any(|case| case == case_preverification)
        || batch.experiment_id_sha256 != public_case.vocabulary.experiment_id_sha256
        || private_case.experiment_id_sha256 != batch.experiment_id_sha256
        || private_case.public_case_root_sha256 != public_case.public_case_root_sha256
        || private_case.case_id_sha256 != case_preverification.case_id_sha256
        || selected_probe.experiment_id_sha256 != private_case.case_id_sha256
        || selected_probe.probe_root_sha256
            != case_preverification
                .tournament
                .tournament_winner_probe_root_sha256
        || safety_request.selection_root_sha256 != case_preverification.receipt_root_sha256
        || safety_request.selected_probe != *selected_probe
        || safety_receipt.safety_request_root_sha256 != safety_request.request_root_sha256
        || safety_receipt.disposition != K2UncertaintyPrivateSafetyDispositionV1::Pass
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_dispatch_boundary_invalid",
        ));
    }
    let resolved_effect = private_case
        .mapping
        .iter()
        .find(|entry| entry.opaque_action_root_sha256 == selected_probe.action_id_sha256)
        .map(|entry| entry.effect.clone())
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_dispatch_private_effect_missing",
        ))?;
    if safety_request.resolved_private_effect != resolved_effect {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_dispatch_safety_effect_mismatch",
        ));
    }
    let worker_request = K2InquiryWorkerRequestV1::seal(
        private_case.case_id_sha256.clone(),
        case_preverification.receipt_root_sha256.clone(),
        selected_probe.probe_root_sha256.clone(),
        selected_probe.action_id_sha256.clone(),
        worker_executable_sha256.to_owned(),
        selected_probe.initial_manifest.clone(),
        resolved_effect.clone(),
    )?;
    let observer_request = K2InquiryObserverRequestV1::seal(
        private_case.case_id_sha256.clone(),
        selected_probe.probe_root_sha256.clone(),
        observer_executable_sha256.to_owned(),
    )?;
    let resolved_effect_root_sha256 = uncertainty_root_v1(&resolved_effect)?;
    let authority = denied_authority_v1();
    let receipt_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_DISPATCH_RECEIPT_SCHEMA_V1,
        &batch.batch_root_sha256,
        &case_preverification.receipt_root_sha256,
        &safety_receipt.receipt_root_sha256,
        &selected_probe.probe_root_sha256,
        &selected_probe.action_id_sha256,
        &resolved_effect_root_sha256,
        &worker_request.request_root_sha256,
        &observer_request.request_root_sha256,
        &authority,
    ))?;
    let receipt = K2UncertaintyDispatchReceiptV1 {
        schema: K2_UNCERTAINTY_DISPATCH_RECEIPT_SCHEMA_V1.to_owned(),
        batch_precommit_root_sha256: batch.batch_root_sha256.clone(),
        case_preverification_root_sha256: case_preverification.receipt_root_sha256.clone(),
        safety_receipt_root_sha256: safety_receipt.receipt_root_sha256.clone(),
        selected_probe_root_sha256: selected_probe.probe_root_sha256.clone(),
        selected_action_root_sha256: selected_probe.action_id_sha256.clone(),
        resolved_effect_root_sha256,
        worker_request_root_sha256: worker_request.request_root_sha256.clone(),
        observer_request_root_sha256: observer_request.request_root_sha256.clone(),
        authority,
        receipt_root_sha256,
    };
    receipt.validate()?;
    Ok(K2UncertaintyDispatchArtifactsV1 {
        resolved_effect,
        worker_request,
        observer_request,
        receipt,
    })
}

pub fn materialize_self_formed_probe_files_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    probe: &K2InquiryProbeV1,
) -> K2CompositionResultV1<BTreeMap<String, Vec<u8>>> {
    public_case.validate()?;
    probe.validate()?;
    let contents = public_case
        .vocabulary
        .content_atoms
        .iter()
        .map(|atom| (atom.bytes_sha256.clone(), atom.bytes.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut files = BTreeMap::new();
    for entry in &probe.initial_manifest.entries {
        let bytes =
            contents
                .get(&entry.content_sha256)
                .cloned()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_dispatch_content_missing",
                ))?;
        if bytes.len() as u64 != entry.byte_len {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_dispatch_content_length_mismatch",
            ));
        }
        files.insert(entry.path.clone(), bytes);
    }
    Ok(files)
}
