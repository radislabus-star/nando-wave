use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path};

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, K2InquiryObservationModeV1,
    K2InquiryWorkerOutcomeV1, composition_sha256_bytes_v1, composition_sha256_file_v1,
};
use super::final_verifier_frontier::{independent_accounting_v1, verify_frontier_v1};
use super::final_verifier_induction::{independent_apply_manifest_v1, verify_induction_v1};
use super::final_verifier_selection::verify_selection_v1;
use super::{
    K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V1, K2_UNCERTAINTY_FRONTIER_SCHEMA_V1,
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_PROBE_OUTPUT_SCHEMA_V1,
    K2_UNCERTAINTY_STATE_UNIVERSE_SCHEMA_V1, K2UncertaintyCaseVerificationReceiptV1,
    K2UncertaintyFinalVerifierRequestV1, K2UncertaintyFrontierPageV1, K2UncertaintyFrontierV1,
    K2UncertaintyPrivateSafetyDispositionV1, K2UncertaintyProbeOutputV1,
    K2UncertaintyStateUniverseV1, K2UncertaintySyntacticModelV1, denied_authority_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

pub fn verify_self_formed_case_independently_v1(
    request: &K2UncertaintyFinalVerifierRequestV1,
    evidence_root: &Path,
) -> K2CompositionResultV1<K2UncertaintyCaseVerificationReceiptV1> {
    request.validate()?;
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
        &request.case_preverification,
    )?;
    let selected = frontier
        .representatives
        .get(
            &request
                .case_preverification
                .tournament
                .tournament_winner_probe_root_sha256,
        )
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_selected_probe_missing",
        ))?;
    verify_private_safety_v1(request, selected, &induction.effects)?;
    verify_dispatch_and_execution_v1(request, selected)?;

    let survivors = selected
        .predictions
        .iter()
        .filter(|prediction| {
            prediction.observable_outcome_root_sha256
                == request.observation.observable_outcome_root_sha256
        })
        .collect::<Vec<_>>();
    let selected_outcome_precommitted = !survivors.is_empty();
    let true_syntax = K2UncertaintySyntacticModelV1::seal(
        request
            .private_case
            .mapping
            .iter()
            .map(|entry| super::super::K2InquiryModelActionV1 {
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
            "self_formed_final_private_syntax_missing",
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
            "self_formed_final_private_class_missing",
        ))?;
    let true_world = learned
        .world_models
        .iter()
        .find(|model| model.model_id_sha256 == true_class.class_root_sha256)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_private_world_missing",
        ))?;
    let private_true_class_match =
        survivors.len() == 1 && survivors[0].model_root_sha256 == true_world.model_root_sha256;
    if !selected_outcome_precommitted || !private_true_class_match {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_semantic_elimination_failed",
        ));
    }
    let mut receipt = K2UncertaintyCaseVerificationReceiptV1 {
        schema: K2_UNCERTAINTY_CASE_VERIFICATION_SCHEMA_V1.to_owned(),
        verifier_executable_sha256: request.verifier_executable_sha256.clone(),
        verifier_request_root_sha256: request.request_root_sha256.clone(),
        case_id_sha256: public_case.vocabulary.case_id_sha256.clone(),
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
        safety_verified: true,
        worker_observer_match: true,
        surviving_semantic_classes: survivors.len() as u64,
        private_true_class_match,
        selected_outcome_precommitted,
        false_accepts: 0,
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    Ok(receipt)
}

pub fn run_self_formed_final_verifier_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_final_verifier_stdin"))?;
    let request: K2UncertaintyFinalVerifierRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_final_verifier"))?;
    if composition_sha256_file_v1(&executable)? != request.verifier_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_verifier_executable_mismatch",
        ));
    }
    let receipt = verify_self_formed_case_independently_v1(&request, Path::new("/evidence"))?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_final_verifier_stdout"))
}

pub(super) fn independent_reopen_frontier_v1(
    probe_request: &super::K2UncertaintyProbeRequestV1,
    probe_artifacts: &super::K2UncertaintyProbeArtifactsV1,
    evidence_root: &Path,
) -> K2CompositionResultV1<K2UncertaintyProbeOutputV1> {
    let mut values = Vec::with_capacity(probe_artifacts.entries.len());
    for entry in &probe_artifacts.entries {
        let relative = Path::new(&entry.relative_path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_artifact_path_invalid",
            ));
        }
        let bytes = fs::read(evidence_root.join(relative))
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_final_artifact"))?;
        if bytes.len() as u64 != entry.byte_len
            || composition_sha256_bytes_v1(&bytes) != entry.content_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_artifact_content_mismatch",
            ));
        }
        values.push(bytes);
    }
    let state: K2UncertaintyStateUniverseV1 = uncertainty_decode_v1(&values[0])?;
    let frontier: K2UncertaintyFrontierV1 = uncertainty_decode_v1(&values[1])?;
    let pages = values[2..]
        .iter()
        .map(|bytes| uncertainty_decode_v1::<K2UncertaintyFrontierPageV1>(bytes))
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    if state.schema != K2_UNCERTAINTY_STATE_UNIVERSE_SCHEMA_V1
        || frontier.schema != K2_UNCERTAINTY_FRONTIER_SCHEMA_V1
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_artifact_schema_invalid",
        ));
    }
    let mut output = K2UncertaintyProbeOutputV1 {
        schema: K2_UNCERTAINTY_PROBE_OUTPUT_SCHEMA_V1.to_owned(),
        probe_request_root_sha256: probe_request.request_root_sha256.clone(),
        state_universe: state,
        pages,
        frontier,
        authority: denied_authority_v1(),
        output_root_sha256: String::new(),
    };
    output.reseal()?;
    if output.state_universe.universe_root_sha256 != probe_artifacts.state_universe_root_sha256
        || output.frontier.frontier_root_sha256 != probe_artifacts.frontier_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_artifact_receipt_mismatch",
        ));
    }
    Ok(output)
}

fn verify_private_safety_v1(
    request: &K2UncertaintyFinalVerifierRequestV1,
    selected: &super::K2UncertaintyRawProbeDispositionV1,
    effects: &[super::super::K2CompositionLearnedEffectV1],
) -> K2CompositionResultV1<()> {
    let resolved = request
        .private_case
        .mapping
        .iter()
        .find(|entry| entry.opaque_action_root_sha256 == selected.probe.action_id_sha256)
        .map(|entry| &entry.effect)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_private_effect_missing",
        ))?;
    let candidates = effects
        .iter()
        .cloned()
        .map(super::K2UncertaintyEffectCandidateV1::seal)
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    let grammar_root =
        uncertainty_root_v1(&("nando.k2-self-formed-effect-grammar.v1", candidates))?;
    let accounting = independent_accounting_v1(
        &request.probe_request.public_case.vocabulary,
        &selected.probe.initial_manifest,
        resolved,
    )?;
    if request.safety_request.selected_probe != selected.probe
        || request.safety_request.resolved_private_effect != *resolved
        || request.safety_request.grammar_root_sha256 != grammar_root
        || request.safety_receipt.disposition != K2UncertaintyPrivateSafetyDispositionV1::Pass
        || request.safety_receipt.selected_effect_accounting.as_ref() != Some(&accounting)
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_private_safety_mismatch",
        ));
    }
    Ok(())
}

fn verify_dispatch_and_execution_v1(
    request: &K2UncertaintyFinalVerifierRequestV1,
    selected: &super::K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<()> {
    let resolved = &request.safety_request.resolved_private_effect;
    let (applied, reason, expected_post) =
        independent_apply_manifest_v1(&selected.probe.initial_manifest, resolved)?;
    let expected_observable = uncertainty_root_v1(&(
        "nando.k2-inquiry-observable-exact-manifest.v1",
        &expected_post,
    ))?;
    let worker: &K2InquiryWorkerOutcomeV1 = &request.worker_outcome;
    if request.worker_request.selected_probe_root_sha256 != selected.probe.probe_root_sha256
        || request.worker_request.selected_action_id_sha256 != selected.probe.action_id_sha256
        || request.worker_request.initial_manifest != selected.probe.initial_manifest
        || request.worker_request.resolved_effect != *resolved
        || request.observer_request.selected_probe_root_sha256 != selected.probe.probe_root_sha256
        || worker.pre_manifest != selected.probe.initial_manifest
        || worker.post_manifest != expected_post
        || worker.transition_applied != applied
        || worker.transition_reason != reason
        || request.observation.post_manifest != expected_post
        || request.observation.observable_outcome_root_sha256 != expected_observable
        || worker.post_manifest != request.observation.post_manifest
        || request.dispatch_receipt.selected_probe_root_sha256 != selected.probe.probe_root_sha256
        || request.dispatch_receipt.selected_action_root_sha256 != selected.probe.action_id_sha256
        || request.dispatch_receipt.resolved_effect_root_sha256 != uncertainty_root_v1(resolved)?
        || selected.probe.observation_mode != K2InquiryObservationModeV1::ExactImmediate
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_execution_mismatch",
        ));
    }
    Ok(())
}
