//! Adaptive post-freeze transfer basis for live operator crystallization.
//!
//! The complete future reservoir is monitoring evidence. Only the smallest
//! deterministic independent subset that closes the executable proof becomes
//! package authority.

use super::induction::{commitment_hex, reextract_live_scalar_circuit_sample};
use super::*;

const TRANSFER_BASIS_MAX_ROWS: usize = 12;

pub(super) struct LiveScalarTransferBasisV1 {
    pub(super) operator: crate::VerifiedCrystallizedOperator,
    pub(super) transitions: Vec<TeacherTransition>,
    pub(super) monitored_exact_rows: usize,
    pub(super) applicability_negative_rows: usize,
    pub(super) censored_rows: usize,
}

enum BasisAttemptV1 {
    Ready(Box<crate::VerifiedCrystallizedOperator>),
    Incomplete(String),
    ApplicabilityNegative(String),
    Contradiction(String),
}

pub(super) fn crystallize_minimal_transfer_basis_v1(
    frozen: &FrozenOperatorBlueprintSet,
    competing: &CompetingBlueprintSet,
    transfer_future: &[TeacherTransition],
) -> Result<LiveScalarTransferBasisV1, String> {
    let actor_hypotheses = competing
        .actors_by_blueprint
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let mut prepared = Vec::new();
    let mut pre_binding_negatives = 0_usize;
    let mut censored_rows = 0_usize;
    for transition in transfer_future {
        match reextract_live_scalar_circuit_sample(transition, &actor_hypotheses) {
            Ok(sample) => prepared.push((transition.clone(), sample)),
            Err(blocker) if reextract_applicability_negative(blocker) => {
                pre_binding_negatives = pre_binding_negatives.saturating_add(1);
            }
            Err(blocker) if reextract_censored(blocker) => {
                censored_rows = censored_rows.saturating_add(1);
            }
            Err(blocker) => {
                return Err(format!("future_reextract_{blocker:?}").to_lowercase());
            }
        }
    }
    prepared.sort_by(|(left, _), (right, _)| {
        left.before
            .observed_at_unix_nanos
            .cmp(&right.before.observed_at_unix_nanos)
            .then_with(|| {
                left.before
                    .frame_id_sha256
                    .cmp(&right.before.frame_id_sha256)
            })
    });

    let monitoring = prepared.clone();
    let mut unique_lineages = BTreeSet::new();
    let independent = prepared
        .into_iter()
        .filter(|(_, sample)| unique_lineages.insert(*sample.bundle.lineage_sha256()))
        .collect::<Vec<_>>();
    if independent.is_empty() {
        return Err("independent_future_basis_empty".to_owned());
    }

    let mut applicability_negative_indices = BTreeSet::new();
    let mut last_incomplete = "transfer_basis_no_full_phase_winner".to_owned();
    for (index, row) in independent.iter().enumerate() {
        match attempt_basis(frozen, competing, &[row]) {
            BasisAttemptV1::Ready(operator) => {
                return finalize_basis(
                    *operator,
                    vec![row.clone()],
                    &monitoring,
                    pre_binding_negatives,
                    censored_rows,
                );
            }
            BasisAttemptV1::ApplicabilityNegative(blocker) => {
                applicability_negative_indices.insert(index);
                last_incomplete = blocker;
            }
            BasisAttemptV1::Incomplete(blocker) => last_incomplete = blocker,
            BasisAttemptV1::Contradiction(blocker) => return Err(blocker),
        }
    }

    let candidates = independent
        .iter()
        .enumerate()
        .filter(|(index, _)| !applicability_negative_indices.contains(index))
        .map(|(_, row)| row)
        .collect::<Vec<_>>();
    let max_rows = candidates.len().min(TRANSFER_BASIS_MAX_ROWS);
    for basis_rows in 2..=max_rows {
        let basis = &candidates[..basis_rows];
        match attempt_basis(frozen, competing, basis) {
            BasisAttemptV1::Ready(operator) => {
                return finalize_basis(
                    *operator,
                    basis.iter().map(|row| (*row).clone()).collect(),
                    &monitoring,
                    pre_binding_negatives,
                    censored_rows,
                );
            }
            BasisAttemptV1::Incomplete(blocker)
            | BasisAttemptV1::ApplicabilityNegative(blocker) => last_incomplete = blocker,
            BasisAttemptV1::Contradiction(blocker) => return Err(blocker),
        }
    }
    Err(last_incomplete)
}

fn attempt_basis(
    frozen: &FrozenOperatorBlueprintSet,
    competing: &CompetingBlueprintSet,
    basis: &[&(TeacherTransition, LiveScalarCircuitSample)],
) -> BasisAttemptV1 {
    let future_evidence = match basis
        .iter()
        .map(|(_, sample)| {
            BlueprintFutureEvidence::new(
                sample.raw_input_sha256,
                sample.extractor_version.max(1),
                sample.bundle.clone(),
            )
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(evidence) => evidence,
        Err(error) => {
            return BasisAttemptV1::Contradiction(
                format!("future_evidence_{error:?}").to_lowercase(),
            );
        }
    };
    let full = BlueprintFutureEvaluator::evaluate_and_seal(
        frozen,
        &future_evidence,
        Default::default(),
        BlueprintPhaseControl::Full,
    );
    let Some(winner) = full.winner_receipt() else {
        return BasisAttemptV1::Incomplete(
            format!("full_phase_no_winner:{:?}", full.report().blocker).to_lowercase(),
        );
    };
    let controls_abstain = [
        BlueprintPhaseControl::NoPhase,
        BlueprintPhaseControl::ShuffledPhase,
        BlueprintPhaseControl::MagnitudeOnly,
        BlueprintPhaseControl::MatchedRandomCenter,
    ]
    .into_iter()
    .all(|control| {
        BlueprintFutureEvaluator::evaluate_and_seal(
            frozen,
            &future_evidence,
            Default::default(),
            control,
        )
        .winner_receipt()
        .is_none()
    });
    if !controls_abstain {
        return BasisAttemptV1::Contradiction("phase_control_selected_winner".to_owned());
    }
    let Some(actor_template) = competing
        .actors_by_blueprint
        .get(winner.winner_sha256())
        .cloned()
    else {
        return BasisAttemptV1::Contradiction("winner_actor_contract_missing".to_owned());
    };
    let mut future_window = frozen.future_window();
    for (_, sample) in basis {
        if let Err(error) = future_window.admit_evidence(&sample.bundle) {
            return BasisAttemptV1::Contradiction(
                format!("future_lineage_{error:?}").to_lowercase(),
            );
        }
    }
    let receipts = basis
        .iter()
        .zip(&future_evidence)
        .map(|((_, sample), evidence)| CrystallizationParityReceipt {
            future_lineage_sha256: *sample.bundle.lineage_sha256(),
            future_surface_sha256: *sample.bundle.surface_sha256(),
            future_bundle_sha256: *evidence.bundle_sha256(),
            raw_input_sha256: sample.raw_input_sha256,
            extractor_version: sample.extractor_version.max(1),
            anchors: sample.anchors.clone(),
            request_text: sample.request_text.clone(),
            provider_payload: sample.provider_payload.clone(),
            expected_response: sample.expected_response.clone(),
        })
        .collect::<Vec<_>>();
    match CrystallizedOperator::crystallize_with_actor_template(
        &future_window,
        winner,
        &future_evidence,
        &receipts,
        actor_template,
    ) {
        Ok(operator) => BasisAttemptV1::Ready(Box::new(operator)),
        Err(error) if applicability_error(error) => BasisAttemptV1::ApplicabilityNegative(
            format!("crystallization_{error:?}").to_lowercase(),
        ),
        Err(error) => {
            BasisAttemptV1::Contradiction(format!("crystallization_{error:?}").to_lowercase())
        }
    }
}

fn finalize_basis(
    operator: crate::VerifiedCrystallizedOperator,
    basis: Vec<(TeacherTransition, LiveScalarCircuitSample)>,
    all_future: &[(TeacherTransition, LiveScalarCircuitSample)],
    pre_binding_negatives: usize,
    censored_rows: usize,
) -> Result<LiveScalarTransferBasisV1, String> {
    let basis_frames = basis
        .iter()
        .map(|(transition, _)| transition.before.frame_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let mut monitored_exact_rows = 0_usize;
    let mut applicability_negative_rows = 0_usize;
    for (transition, sample) in all_future {
        if basis_frames.contains(transition.before.frame_id_sha256.as_str()) {
            continue;
        }
        let bound = match operator.bind_pre_action(&sample.request_text, &sample.provider_payload) {
            Ok(bound) => bound,
            Err(error) if applicability_error(error) => {
                applicability_negative_rows = applicability_negative_rows.saturating_add(1);
                continue;
            }
            Err(error) => {
                return Err(format!("future_monitor_bind_{error:?}").to_lowercase());
            }
        };
        let actual = bound
            .execute_verified()
            .map_err(|error| format!("future_monitor_execute_{error:?}").to_lowercase())?;
        if actual != sample.expected_response
            && !crate::online_admission::responses_match_after_execution_budget_normalization(
                &actual,
                &sample.expected_response,
            )
        {
            return Err(format!(
                "future_monitor_response_mismatch:{}",
                commitment_hex(sample.bundle.surface_sha256())
            ));
        }
        monitored_exact_rows = monitored_exact_rows.saturating_add(1);
    }
    Ok(LiveScalarTransferBasisV1 {
        operator,
        transitions: basis
            .into_iter()
            .map(|(transition, _)| transition)
            .collect(),
        monitored_exact_rows,
        applicability_negative_rows: applicability_negative_rows
            .saturating_add(pre_binding_negatives),
        censored_rows,
    })
}

const fn reextract_applicability_negative(blocker: LiveScalarShadowBlocker) -> bool {
    matches!(
        blocker,
        LiveScalarShadowBlocker::NoExactSourceNeutralProgram
            | LiveScalarShadowBlocker::CanonicalCandidateMissing
            | LiveScalarShadowBlocker::ObservedRoleExtractionFailed
            | LiveScalarShadowBlocker::TeacherProgramRoleValueUnavailable
            | LiveScalarShadowBlocker::TeacherProgramRoleValueNotObserved
            | LiveScalarShadowBlocker::TeacherProgramRoleValueCandidateMismatch
            | LiveScalarShadowBlocker::TeacherProgramRoleValueAbsentFromPayload
    )
}

const fn reextract_censored(blocker: LiveScalarShadowBlocker) -> bool {
    matches!(
        blocker,
        LiveScalarShadowBlocker::MissingParityCase
            | LiveScalarShadowBlocker::PayloadTooLarge
            | LiveScalarShadowBlocker::PayloadSerializationFailed
            | LiveScalarShadowBlocker::RequestTextInvalid
            | LiveScalarShadowBlocker::ProviderInputMissing
    )
}

const fn applicability_error(error: crate::CrystallizedOperatorError) -> bool {
    matches!(
        error,
        crate::CrystallizedOperatorError::RuntimeBindingExhausted
            | crate::CrystallizedOperatorError::RuntimeRelationMismatch
            | crate::CrystallizedOperatorError::MissingRuntimeAnchor
            | crate::CrystallizedOperatorError::RuntimeOperandArityMismatch
            | crate::CrystallizedOperatorError::RuntimeOperandTypeMismatch
            | crate::CrystallizedOperatorError::AmbiguousRuntimeAction
            | crate::CrystallizedOperatorError::ActorDidNotExecute
    )
}
