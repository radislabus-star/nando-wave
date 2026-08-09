//! Independent reconstruction of selected Raw Phase executable hypotheses.

use std::collections::BTreeSet;

use nando_core::wave::{
    BlueprintFutureEvaluator, BlueprintFutureEvidence, BlueprintPhaseControl, FrozenBlueprintError,
};
use nando_operator_learning::multi_source::{
    MultiSourceT1ProofBasisV1, RawPhaseExecutableEvidenceV1, RawPhaseSelectedExecutableReceiptV1,
    raw_phase_executable_runtime_selectors_v1, raw_phase_executable_surface_bundle_v1,
    rebuild_raw_phase_selected_executable_v1,
};
use nando_operator_learning::{CandidateFreezeReceiptV1, RuntimeFrame};

use super::induction::{commitment_hex, digest_parts, extractor_version};
use super::*;

pub(super) struct RawPhaseTransferBasisV1 {
    pub(super) operator: crate::VerifiedCrystallizedOperator,
    pub(super) support: Vec<TeacherTransition>,
    pub(super) future: Vec<TeacherTransition>,
}

struct PreparedRawPhaseRowV1 {
    transition: TeacherTransition,
    bundle: SurfaceFragmentBundle,
    anchors: Box<[RuntimeRoleAnchor]>,
    raw_input_sha256: [u8; 32],
    extractor_version: u32,
}

pub(super) fn crystallize_raw_phase_transfer_v1(
    receipt: &RawPhaseSelectedExecutableReceiptV1,
    freeze: &CandidateFreezeReceiptV1,
    program: &ResponseProgram,
    proof_basis: &MultiSourceT1ProofBasisV1,
    transitions: &[TeacherTransition],
) -> Result<RawPhaseTransferBasisV1, String> {
    let rebuilt = rebuild_raw_phase_selected_executable_v1(receipt, freeze, program)
        .map_err(|blocker| format!("multi_source_{blocker}"))?;
    if proof_basis.raw_phase_future_evidence.is_empty() {
        return Err("multi_source_raw_phase_future_evidence_missing".to_owned());
    }

    let support = receipt
        .support_evidence
        .iter()
        .map(|evidence| prepare_raw_phase_row(program, evidence, transitions))
        .collect::<Result<Vec<_>, _>>()?;
    let support_bundles = support
        .iter()
        .map(|row| row.bundle.clone())
        .collect::<Vec<_>>();
    if support_bundles != rebuilt.support_bundles {
        return Err("multi_source_raw_phase_support_reconstruction_mismatch".to_owned());
    }

    let future = proof_basis
        .raw_phase_future_evidence
        .iter()
        .map(|evidence| prepare_raw_phase_row(program, evidence, transitions))
        .collect::<Result<Vec<_>, _>>()?;
    let support_lineages = support
        .iter()
        .map(|row| *row.bundle.lineage_sha256())
        .collect::<BTreeSet<_>>();
    let future_lineages = future
        .iter()
        .map(|row| *row.bundle.lineage_sha256())
        .collect::<BTreeSet<_>>();
    if future_lineages.len() != future.len() || !support_lineages.is_disjoint(&future_lineages) {
        return Err("multi_source_raw_phase_future_lineage_invalid".to_owned());
    }

    let future_evidence = future
        .iter()
        .map(|row| {
            BlueprintFutureEvidence::new(
                row.raw_input_sha256,
                row.extractor_version,
                row.bundle.clone(),
            )
            .map_err(|error| format!("multi_source_raw_phase_future_{error:?}").to_lowercase())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let full = BlueprintFutureEvaluator::evaluate_and_seal(
        &rebuilt.frozen,
        &future_evidence,
        Default::default(),
        BlueprintPhaseControl::Full,
    );
    let winner = full.winner_receipt().ok_or_else(|| {
        format!(
            "multi_source_raw_phase_full_no_winner:{:?}",
            full.report().blocker
        )
        .to_lowercase()
    })?;
    let winner_root = commitment_hex(winner.winner_sha256());
    if receipt
        .selected_disposition
        .blueprint_fingerprints_sha256
        .binary_search(&winner_root)
        .is_err()
    {
        return Err("multi_source_raw_phase_winner_not_in_selected_receipt".to_owned());
    }
    let controls_abstain = [
        BlueprintPhaseControl::NoPhase,
        BlueprintPhaseControl::ShuffledPhase,
        BlueprintPhaseControl::MagnitudeOnly,
        BlueprintPhaseControl::MatchedRandomCenter,
    ]
    .into_iter()
    .all(|control| {
        BlueprintFutureEvaluator::evaluate_and_seal(
            &rebuilt.frozen,
            &future_evidence,
            Default::default(),
            control,
        )
        .winner_receipt()
        .is_none()
    });
    if !controls_abstain {
        return Err("multi_source_raw_phase_control_selected_winner".to_owned());
    }

    let mut future_window = rebuilt.frozen.future_window();
    for row in &future {
        future_window
            .admit_evidence(&row.bundle)
            .map_err(raw_phase_future_window_blocker)?;
    }
    let parity_receipts = future
        .iter()
        .zip(&future_evidence)
        .map(|(row, evidence)| {
            let parity = row
                .transition
                .runtime_parity_case
                .as_ref()
                .expect("prepared Raw Phase row owns parity");
            CrystallizationParityReceipt {
                future_lineage_sha256: *row.bundle.lineage_sha256(),
                future_surface_sha256: *row.bundle.surface_sha256(),
                future_bundle_sha256: *evidence.bundle_sha256(),
                raw_input_sha256: row.raw_input_sha256,
                extractor_version: row.extractor_version,
                anchors: row.anchors.clone(),
                request_text: parity.request_text.clone(),
                provider_payload: parity.provider_payload.clone(),
                expected_response: parity.expected_response.clone(),
            }
        })
        .collect::<Vec<_>>();
    let operator = CrystallizedOperator::crystallize_with_actor_template(
        &future_window,
        winner,
        &future_evidence,
        &parity_receipts,
        program.clone(),
    )
    .map_err(|error| format!("multi_source_raw_phase_crystallization_{error:?}").to_lowercase())?;

    Ok(RawPhaseTransferBasisV1 {
        operator,
        support: support.into_iter().map(|row| row.transition).collect(),
        future: future.into_iter().map(|row| row.transition).collect(),
    })
}

fn prepare_raw_phase_row(
    program: &ResponseProgram,
    evidence: &RawPhaseExecutableEvidenceV1,
    transitions: &[TeacherTransition],
) -> Result<PreparedRawPhaseRowV1, String> {
    evidence
        .validate()
        .map_err(|blocker| format!("multi_source_{blocker}"))?;
    let mut matches = transitions
        .iter()
        .filter(|transition| transition_matches_raw_phase_evidence(transition, evidence));
    let transition = matches.next().cloned().ok_or_else(|| {
        format!(
            "multi_source_raw_phase_runtime_case_missing:{}",
            evidence.frame.frame_id_sha256
        )
    })?;
    if matches.next().is_some() {
        return Err(format!(
            "multi_source_raw_phase_runtime_case_ambiguous:{}",
            evidence.frame.frame_id_sha256
        ));
    }
    validate_transition_binding(program, evidence, &transition)?;
    let bundle = raw_phase_executable_surface_bundle_v1(program, evidence)
        .map_err(|error| format!("multi_source_raw_phase_surface_{error:?}").to_lowercase())?;
    let anchors = raw_phase_runtime_anchors(program, evidence)?;
    if anchors.len() != bundle.relations().len()
        || anchors
            .iter()
            .zip(bundle.relations())
            .any(|(anchor, relation)| anchor.local_role != relation.target_local_role)
    {
        return Err("multi_source_raw_phase_anchor_surface_mismatch".to_owned());
    }
    let parity = transition
        .runtime_parity_case
        .as_ref()
        .ok_or_else(|| "multi_source_raw_phase_runtime_parity_missing".to_owned())?;
    let payload = serde_json::to_vec(&parity.provider_payload)
        .map_err(|_| "multi_source_raw_phase_payload_encoding_failed".to_owned())?;
    let raw_input_sha256 = digest_parts(
        b"nando.live-scalar-raw-input.v1",
        &[parity.request_text.as_bytes(), &payload],
    );
    let extractor_version = extractor_version(&evidence.frame.extractor_version).max(1);
    Ok(PreparedRawPhaseRowV1 {
        transition,
        bundle,
        anchors,
        raw_input_sha256,
        extractor_version,
    })
}

fn transition_matches_raw_phase_evidence(
    transition: &TeacherTransition,
    evidence: &RawPhaseExecutableEvidenceV1,
) -> bool {
    transition
        .runtime_parity_case
        .as_ref()
        .and_then(|parity| parity.capture_receipt.as_ref())
        .and_then(|receipt| receipt.transition_binding.as_ref())
        .is_some_and(|binding| binding.frame_id_sha256 == evidence.frame.frame_id_sha256)
}

fn validate_transition_binding(
    program: &ResponseProgram,
    evidence: &RawPhaseExecutableEvidenceV1,
    transition: &TeacherTransition,
) -> Result<(), String> {
    let parity = transition
        .runtime_parity_case
        .as_ref()
        .ok_or_else(|| "multi_source_raw_phase_runtime_parity_missing".to_owned())?;
    let capture = parity
        .capture_receipt
        .as_ref()
        .ok_or_else(|| "multi_source_raw_phase_capture_receipt_missing".to_owned())?;
    capture
        .validate()
        .map_err(|blocker| format!("multi_source_raw_phase_{blocker}"))?;
    let binding = capture
        .transition_binding
        .as_ref()
        .ok_or_else(|| "multi_source_raw_phase_capture_binding_missing".to_owned())?;
    let rebuilt_outcome = crate::teacher_outcome_from_completed(&evidence.frame)
        .map_err(|_| "multi_source_raw_phase_completed_outcome_invalid".to_owned())?;
    let economics = transition
        .economics
        .as_ref()
        .ok_or_else(|| "multi_source_raw_phase_economics_missing".to_owned())?;
    if binding.sequence != evidence.joined.capture_sequence
        || binding.frame_id_sha256 != evidence.frame.frame_id_sha256
        || parity.evidence_ref_sha256 != evidence.frame.frame_id_sha256
        || transition
            .verify_capture_frame_id(&evidence.frame.frame_id_sha256)
            .is_err()
        || transition.before != RuntimeFrame::from_completed(&evidence.frame)
        || transition.outcome != rebuilt_outcome
        || economics.exact_input_tokens != evidence.joined.input_tokens
        || !economics.ordinary
        || economics.controlled
        || economics.replay
        || evidence.frame.estimated_input_tokens != evidence.joined.input_tokens
    {
        return Err("multi_source_raw_phase_transition_binding_mismatch".to_owned());
    }
    let execution = execute_response(program, &parity.request_text, &parity.provider_payload);
    if execution.status != crate::ResponseExecutionStatus::Executed
        || execution.response.as_deref() != Some(parity.expected_response.as_str())
    {
        return Err("multi_source_raw_phase_actor_outcome_mismatch".to_owned());
    }
    Ok(())
}

fn raw_phase_runtime_anchors(
    program: &ResponseProgram,
    evidence: &RawPhaseExecutableEvidenceV1,
) -> Result<Box<[RuntimeRoleAnchor]>, String> {
    let selectors = raw_phase_executable_runtime_selectors_v1(program, evidence)
        .map_err(|error| format!("multi_source_raw_phase_selectors_{error:?}").to_lowercase())?;
    if selectors.is_empty() || selectors.len() > 16 {
        return Err("multi_source_raw_phase_anchor_count_invalid".to_owned());
    }
    selectors
        .into_iter()
        .enumerate()
        .map(|(index, selector)| {
            Ok(RuntimeRoleAnchor {
                local_role: u8::try_from(index.saturating_add(1))
                    .map_err(|_| "multi_source_raw_phase_anchor_count_invalid".to_owned())?,
                selector,
                json_path_sha256: None,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn raw_phase_future_window_blocker(error: FrozenBlueprintError) -> String {
    format!("multi_source_raw_phase_future_window_{error:?}").to_lowercase()
}
