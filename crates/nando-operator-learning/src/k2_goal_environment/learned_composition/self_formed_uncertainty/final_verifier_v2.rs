use std::io::{Read, Write};
use std::path::Path;

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, K2InquiryModelActionV1,
    K2InquiryObservationModeV1, composition_sha256_file_v1,
};
use super::final_verifier::independent_reopen_frontier_v1;
use super::final_verifier_frontier::{independent_accounting_v1, verify_frontier_v1};
use super::final_verifier_induction::{independent_apply_manifest_v1, verify_induction_v1};
use super::final_verifier_selection::verify_selection_v1;
use super::final_verifier_v2_closure::verify_closure_v2;
use super::{
    K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V2, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2UncertaintyBatchPrecommitV2, K2UncertaintyCasePreverificationV2,
    K2UncertaintyCaseVerificationReceiptV2, K2UncertaintyFinalVerifierRequestV2,
    K2UncertaintyPrivateSafetyDispositionV1, K2UncertaintyRawProbeDispositionV1,
    K2UncertaintySyntacticModelV1, denied_authority_v1,
    resolve_self_formed_final_verifier_material_v2, uncertainty_bytes_v1, uncertainty_decode_v1,
    uncertainty_root_v1,
};

pub fn verify_self_formed_case_independently_v2(
    request: &K2UncertaintyFinalVerifierRequestV2,
    evidence_root: &Path,
) -> K2CompositionResultV1<K2UncertaintyCaseVerificationReceiptV2> {
    request.validate()?;
    let material =
        resolve_self_formed_final_verifier_material_v2(evidence_root, &request.material)?;
    verify_material_bindings_v2(
        request,
        &material.batch_precommit,
        &material.case_preverification,
    )?;
    let output = independent_reopen_frontier_v1(
        &request.probe_request,
        &request.probe_artifacts,
        evidence_root,
    )?;
    let public_case = &request.probe_request.public_case;
    let learned = &request.probe_request.learner_response;
    let induction = verify_induction_v1(public_case, learned)?;
    let frontier = verify_frontier_v1(
        public_case,
        learned,
        &induction,
        &output,
        &request.probe_request.split_commitment_root_sha256,
    )?;
    let selection = verify_selection_v1(
        public_case,
        learned,
        &frontier,
        &output.frontier.frontier_root_sha256,
        &request.probe_request.split_commitment_root_sha256,
        &material.case_preverification.selection_preverification,
    )?;
    let closure = verify_closure_v2(&frontier, &material.case_preverification)?;
    verify_plan_execution_v2(request, &frontier, &induction.effects)?;

    let survivors = learned
        .world_models
        .iter()
        .filter(|model| {
            request
                .dispatch
                .items
                .iter()
                .zip(&request.observation_vector.executions)
                .all(|(item, execution)| {
                    frontier
                        .representatives
                        .get(&item.selected_probe.probe_root_sha256)
                        .and_then(|selected| {
                            selected.predictions.iter().find(|prediction| {
                                prediction.model_root_sha256 == model.model_root_sha256
                            })
                        })
                        .is_some_and(|prediction| {
                            prediction.observable_outcome_root_sha256
                                == execution.observation.observable_outcome_root_sha256
                        })
                })
        })
        .collect::<Vec<_>>();
    let true_syntax = K2UncertaintySyntacticModelV1::seal(
        request
            .private_case
            .mapping
            .iter()
            .map(|entry| K2InquiryModelActionV1 {
                action_id_sha256: entry.opaque_action_root_sha256.clone(),
                effect: entry.effect.clone(),
            })
            .collect(),
    )?;
    if !induction
        .syntactic_models
        .iter()
        .any(|model| model == &true_syntax)
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_private_syntax_missing",
        ));
    }
    let true_class = learned
        .model_set
        .semantic_classes
        .iter()
        .find(|class| {
            class
                .syntax_member_roots_sha256
                .contains(&true_syntax.syntax_root_sha256)
        })
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_private_class_missing",
        ))?;
    let true_world = learned
        .world_models
        .iter()
        .find(|model| model.model_id_sha256 == true_class.class_root_sha256)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_private_world_missing",
        ))?;
    let private_true_class_match =
        survivors.len() == 1 && survivors[0].model_root_sha256 == true_world.model_root_sha256;
    if !private_true_class_match {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_joint_elimination_failed",
        ));
    }

    let execution_count = request.observation_vector.executions.len() as u64;
    let mut receipt = K2UncertaintyCaseVerificationReceiptV2 {
        schema: K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V2.to_owned(),
        verifier_executable_sha256: request.verifier_executable_sha256.clone(),
        verifier_request_root_sha256: request.request_root_sha256.clone(),
        case_id_sha256: public_case.vocabulary.case_id_sha256.clone(),
        closure_plan_root_sha256: request.dispatch.closure_plan.plan_root_sha256.clone(),
        observation_vector_root_sha256: request.observation_vector.vector_root_sha256.clone(),
        consistency_dispositions: learned.consistency.dispositions.len() as u64,
        materialized_models: induction.syntactic_models.len() as u64,
        semantic_signature_outcomes: learned
            .model_set
            .semantic_signatures
            .iter()
            .map(|signature| signature.observable_outcome_roots_sha256.len() as u64)
            .sum(),
        raw_probe_dispositions: output.frontier.raw_probe_count,
        raw_predictions: output.frontier.raw_prediction_count,
        representative_count: selection.representative_count,
        tournament_requests: selection.tournament_requests,
        adapted_predictions: selection.adapted_predictions,
        completion_candidate_count: closure.candidate_count,
        joint_pairwise_comparisons: closure.joint_pairwise_comparisons,
        selected_probe_executions: execution_count,
        safety_verified: execution_count,
        worker_observer_matches: execution_count,
        surviving_semantic_classes: survivors.len() as u64,
        private_true_class_match,
        ordered_outcomes_precommitted: true,
        false_accepts: 0,
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    Ok(receipt)
}

fn verify_material_bindings_v2(
    request: &K2UncertaintyFinalVerifierRequestV2,
    batch: &K2UncertaintyBatchPrecommitV2,
    case: &K2UncertaintyCasePreverificationV2,
) -> K2CompositionResultV1<()> {
    batch.validate()?;
    case.validate()?;
    let case_id = &request.probe_request.public_case.vocabulary.case_id_sha256;
    let entry = batch
        .cases
        .iter()
        .find(|entry| &entry.case_id_sha256 == case_id)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_batch_case_missing",
        ))?;
    let plan = case
        .closure_plan
        .as_ref()
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_closure_unavailable",
        ))?;
    if !batch.dispatch_permitted
        || batch.experiment_id_sha256
            != request
                .probe_request
                .public_case
                .vocabulary
                .experiment_id_sha256
        || request.private_case.experiment_id_sha256 != batch.experiment_id_sha256
        || case.selection_preverification.case_id_sha256 != *case_id
        || entry.case_preverification_root_sha256 != case.receipt_root_sha256
        || entry.closure_plan_root_sha256.as_deref() != Some(plan.plan_root_sha256.as_str())
        || request.dispatch.batch_precommit_root_sha256 != batch.batch_root_sha256
        || request.dispatch.case_preverification_root_sha256 != case.receipt_root_sha256
        || request.dispatch.closure_plan != *plan
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_material_binding_invalid",
        ));
    }
    Ok(())
}

pub fn run_self_formed_final_verifier_process_v2() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_final_verifier_v2_stdin"))?;
    let request: K2UncertaintyFinalVerifierRequestV2 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_final_verifier_v2"))?;
    if composition_sha256_file_v1(&executable)? != request.verifier_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_verifier_v2_executable_mismatch",
        ));
    }
    let receipt = verify_self_formed_case_independently_v2(&request, Path::new("/evidence"))?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_final_verifier_v2_stdout"))
}

fn verify_plan_execution_v2(
    request: &K2UncertaintyFinalVerifierRequestV2,
    frontier: &super::final_verifier_frontier::IndependentFrontierV1,
    effects: &[super::super::K2CompositionLearnedEffectV1],
) -> K2CompositionResultV1<()> {
    let candidates = effects
        .iter()
        .cloned()
        .map(super::K2UncertaintyEffectCandidateV1::seal)
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    let grammar_root =
        uncertainty_root_v1(&("nando.k2-self-formed-effect-grammar.v1", candidates))?;
    for (ordinal, (item, execution)) in request
        .dispatch
        .items
        .iter()
        .zip(&request.observation_vector.executions)
        .enumerate()
    {
        let selected: &K2UncertaintyRawProbeDispositionV1 = frontier
            .representatives
            .get(&item.selected_probe.probe_root_sha256)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_final_v2_selected_probe_missing",
            ))?;
        let resolved = request
            .private_case
            .mapping
            .iter()
            .find(|entry| entry.opaque_action_root_sha256 == selected.probe.action_id_sha256)
            .map(|entry| &entry.effect)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_final_v2_private_effect_missing",
            ))?;
        let accounting = independent_accounting_v1(
            &request.probe_request.public_case.vocabulary,
            &selected.probe.initial_manifest,
            resolved,
        )?;
        let safety = &item.safety_receipt;
        if item.selected_probe != selected.probe
            || item.safety_request.selected_probe != selected.probe
            || item.safety_request.resolved_private_effect != *resolved
            || item.safety_request.grammar_root_sha256 != grammar_root
            || safety.disposition != K2UncertaintyPrivateSafetyDispositionV1::Pass
            || safety.selected_effect_accounting.as_ref() != Some(&accounting)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_v2_private_safety_mismatch",
            ));
        }
        let (applied, reason, expected_post) =
            independent_apply_manifest_v1(&selected.probe.initial_manifest, resolved)?;
        let expected_observable = uncertainty_root_v1(&(
            "nando.k2-inquiry-observable-exact-manifest.v1",
            &expected_post,
        ))?;
        let expected_prediction_roots = selected
            .predictions
            .iter()
            .map(|prediction| prediction.prediction_root_sha256.clone())
            .collect::<Vec<_>>();
        if request
            .dispatch
            .closure_plan
            .ordered_prediction_roots_sha256[ordinal]
            != expected_prediction_roots
            || item.worker_request.initial_manifest != selected.probe.initial_manifest
            || item.worker_request.resolved_effect != *resolved
            || execution.worker_outcome.pre_manifest != selected.probe.initial_manifest
            || execution.worker_outcome.post_manifest != expected_post
            || execution.worker_outcome.transition_applied != applied
            || execution.worker_outcome.transition_reason != reason
            || execution.observation.post_manifest != expected_post
            || execution.observation.observable_outcome_root_sha256 != expected_observable
            || selected.probe.observation_mode != K2InquiryObservationModeV1::ExactImmediate
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_v2_execution_mismatch",
            ));
        }
    }
    Ok(())
}
