//! Bounded live-shadow state, phase evaluation, and candidate assembly.
//!
//! Samples are compiled by `induction`; this owner only advances frozen state.

use super::identification::identify_live_scalar_law_v1;
use super::induction::{
    canonicalize_scalar_program_roles, commitment_hex, extract_live_scalar_circuit_sample,
    observed_rich_scalar_surface, parse_commitment, program_has_filter_count,
    program_transform_flags, program_transform_opcode, reextract_live_scalar_circuit_sample,
    rich_scalar_program_roles, scalar_program_role_slot_types, source_neutral_scalar_program_shape,
};
use super::*;

impl LiveScalarShadowState {
    pub fn observe(&mut self, transition: &TeacherTransition) {
        self.observations = self.observations.saturating_add(1);
        let sample = match extract_live_scalar_circuit_sample(transition) {
            Ok(sample) => sample,
            Err(blocker) => {
                *self.blockers.entry(blocker).or_default() += 1;
                let action = crate::teacher_action_symbol(&transition.as_training_relation_frame());
                *self
                    .extraction_blockers_by_action
                    .entry(action)
                    .or_default()
                    .entry(blocker)
                    .or_default() += 1;
                return;
            }
        };
        self.executable = self.executable.saturating_add(1);
        let law_key = commitment_hex(&sample.law_sha256);
        let law = self.laws.entry(law_key.clone()).or_default();
        if law
            .support
            .iter()
            .chain(&law.future)
            .any(|row| row.before.frame_id_sha256 == transition.before.frame_id_sha256)
        {
            self.duplicate_rows = self.duplicate_rows.saturating_add(1);
            return;
        }
        if identify_live_scalar_law_v1(&law_key, law).is_err() || !live_scalar_support_compiles(law)
        {
            if law.support.len() >= LIVE_SCALAR_MAX_EVIDENCE_ROWS {
                *self
                    .blockers
                    .entry(LiveScalarShadowBlocker::HistoricalSupportCapacityReached)
                    .or_default() += 1;
                return;
            }
            if let Err(blocker) = update_support_actor_hypotheses(law, &sample.actor_hypotheses) {
                *self.blockers.entry(blocker).or_default() += 1;
                return;
            }
            law.support.push(transition.clone());
            return;
        }
        if law
            .support
            .iter()
            .any(|row| row.before.session_id_sha256 == transition.before.session_id_sha256)
        {
            *self
                .blockers
                .entry(LiveScalarShadowBlocker::SupportFutureSessionOverlap)
                .or_default() += 1;
            return;
        }
        if law.future.len() < LIVE_SCALAR_MAX_EVIDENCE_ROWS {
            law.future.push(transition.clone());
        } else {
            *self
                .blockers
                .entry(LiveScalarShadowBlocker::FutureCapacityReached)
                .or_default() += 1;
        }
    }

    /// Accepts only a monotonic provenance upgrade for a frame already owned
    /// by the general miner. Learning semantics and physical action stay fixed.
    pub(crate) fn observe_capture_bound_duplicate(&mut self, transition: &TeacherTransition) {
        if !capture_lineage_is_reconstructible(transition) {
            return;
        }
        let sample = match extract_live_scalar_circuit_sample(transition) {
            Ok(sample) => sample,
            Err(blocker) => {
                *self.blockers.entry(blocker).or_default() += 1;
                return;
            }
        };
        let law_key = commitment_hex(&sample.law_sha256);
        let existing = self.laws.get_mut(&law_key).and_then(|law| {
            law.support
                .iter_mut()
                .chain(&mut law.future)
                .find(|row| row.before.frame_id_sha256 == transition.before.frame_id_sha256)
        });
        let Some(existing) = existing else {
            self.observe(transition);
            return;
        };

        self.observations = self.observations.saturating_add(1);
        self.executable = self.executable.saturating_add(1);
        self.duplicate_rows = self.duplicate_rows.saturating_add(1);
        if capture_lineage_is_reconstructible(existing) {
            return;
        }
        let Some(incoming) = transition.runtime_parity_case.as_ref() else {
            return;
        };
        match existing.runtime_parity_case.as_mut() {
            Some(current)
                if current.request_text == incoming.request_text
                    && current.provider_payload == incoming.provider_payload
                    && current.expected_response == incoming.expected_response =>
            {
                current.evidence_ref_sha256 = incoming.evidence_ref_sha256.clone();
                current.capture_receipt = incoming.capture_receipt.clone();
            }
            None => existing.runtime_parity_case = Some(incoming.clone()),
            Some(_) => {
                *self
                    .blockers
                    .entry(LiveScalarShadowBlocker::CaptureProvenanceConflict)
                    .or_default() += 1;
            }
        }
    }

    /// Rebuilds bounded support after a strategy upgrade without reclassifying
    /// historical receipts as post-freeze future evidence.
    pub(crate) fn observe_historical_support(&mut self, transition: &TeacherTransition) {
        self.observations = self.observations.saturating_add(1);
        let sample = match extract_live_scalar_circuit_sample(transition) {
            Ok(sample) => sample,
            Err(blocker) => {
                *self.blockers.entry(blocker).or_default() += 1;
                return;
            }
        };
        self.executable = self.executable.saturating_add(1);
        let law_key = commitment_hex(&sample.law_sha256);
        let law = self.laws.entry(law_key).or_default();
        if law
            .support
            .iter()
            .any(|row| row.before.frame_id_sha256 == transition.before.frame_id_sha256)
        {
            self.duplicate_rows = self.duplicate_rows.saturating_add(1);
            return;
        }
        if law.support.len() >= LIVE_SCALAR_MAX_EVIDENCE_ROWS {
            *self
                .blockers
                .entry(LiveScalarShadowBlocker::HistoricalSupportCapacityReached)
                .or_default() += 1;
            return;
        }
        if let Err(blocker) = update_support_actor_hypotheses(law, &sample.actor_hypotheses) {
            *self.blockers.entry(blocker).or_default() += 1;
            return;
        }
        law.support.push(transition.clone());
    }

    /// Imports a replay row only when the capture owner can reconstruct its
    /// immutable source lineage. Historical evidence can narrow the version
    /// space, but it can never satisfy the post-freeze transfer proof.
    pub(crate) fn observe_capture_bound_historical_support(
        &mut self,
        transition: &TeacherTransition,
    ) {
        if capture_lineage_is_reconstructible(transition) {
            self.observe_historical_support(transition);
        }
    }

    #[must_use]
    pub fn report(&self) -> LiveScalarShadowReport {
        let support_rows = self.laws.values().map(|law| law.support.len()).sum();
        let future_rows = self.laws.values().map(|law| law.future.len()).sum();
        let ingest_blocker_rows = self.blockers.values().copied().sum::<usize>();
        let mut laws = self
            .laws
            .iter()
            .map(|(law_sha256, law)| LiveScalarLawReport {
                law_sha256: law_sha256.clone(),
                teacher_action_symbol: law
                    .support
                    .first()
                    .map(|row| row.outcome.action.action_symbol.clone())
                    .unwrap_or_default(),
                operation_kind: law
                    .support_actor_hypotheses
                    .first()
                    .map(response_operation_kind)
                    .unwrap_or("unresolved")
                    .to_owned(),
                support_rows: law.support.len(),
                future_rows: law.future.len(),
                distinct_support_sessions: law
                    .support
                    .iter()
                    .map(|row| row.before.session_id_sha256.as_str())
                    .collect::<BTreeSet<_>>()
                    .len(),
                actor_hypotheses: law.support_actor_hypotheses.len(),
            })
            .collect::<Vec<_>>();
        laws.sort_by(|left, right| {
            right
                .support_rows
                .cmp(&left.support_rows)
                .then_with(|| left.law_sha256.cmp(&right.law_sha256))
        });
        let mut report = LiveScalarShadowReport {
            identification_policy: "adaptive_version_space_v1".to_owned(),
            observations: self.observations,
            executable: self.executable,
            duplicate_rows: self.duplicate_rows,
            law_count: self.laws.len(),
            support_rows,
            future_rows,
            ingest_accounting_complete: self.observations
                == support_rows
                    .saturating_add(future_rows)
                    .saturating_add(self.duplicate_rows)
                    .saturating_add(ingest_blocker_rows),
            laws,
            blockers: self
                .blockers
                .iter()
                .map(|(blocker, count)| (format!("{blocker:?}").to_lowercase(), *count))
                .collect(),
            extraction_blockers_by_action: self
                .extraction_blockers_by_action
                .iter()
                .map(|(action, blockers)| {
                    (
                        action.clone(),
                        blockers
                            .iter()
                            .map(|(blocker, count)| (format!("{blocker:?}").to_lowercase(), *count))
                            .collect(),
                    )
                })
                .collect(),
            ..LiveScalarShadowReport::default()
        };
        let mut candidates = Vec::new();
        for (law_key, law) in &self.laws {
            evaluate_live_law(law_key, law, &mut report, &mut candidates);
        }
        report.admission_candidates = candidates.len();
        report
    }

    #[must_use]
    pub fn admission_candidates(&self) -> Vec<LiveScalarAdmissionCandidate> {
        let mut report = LiveScalarShadowReport::default();
        let mut candidates = Vec::new();
        for (law_key, law) in &self.laws {
            evaluate_live_law(law_key, law, &mut report, &mut candidates);
        }
        candidates
    }

    /// Returns only already completed support rows for strategy migration.
    /// Frozen future belongs to the old generation and must never be replayed
    /// into a new strategy as either support or future authority.
    pub(crate) fn historical_support_transitions(&self) -> Vec<TeacherTransition> {
        let mut support = self
            .laws
            .values()
            .flat_map(|law| law.support.iter().cloned())
            .map(|transition| (transition.before.frame_id_sha256.clone(), transition))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect::<Vec<_>>();
        support.sort_by(|left, right| {
            left.before
                .observed_at_unix_nanos
                .cmp(&right.before.observed_at_unix_nanos)
                .then_with(|| {
                    left.before
                        .frame_id_sha256
                        .cmp(&right.before.frame_id_sha256)
                })
        });
        support
    }
}

pub(super) fn response_operation_kind(program: &ResponseProgram) -> &'static str {
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles { .. } => "function_call",
        ResponseOperation::CustomToolCallFromRoles { .. } => "custom_tool_call",
        ResponseOperation::ProjectSelectedValue { .. } => "project",
        ResponseOperation::ProjectStatus { .. } => "status",
        ResponseOperation::ComposeCollection { steps, .. } => {
            let has_count = steps
                .iter()
                .any(|step| matches!(step, CollectionProgramStep::Count));
            let has_filter = steps.iter().any(|step| {
                matches!(
                    step,
                    CollectionProgramStep::FilterUniqueFieldEquals { .. }
                        | CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
                        | CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { .. }
                        | CollectionProgramStep::FilterFieldEquals { .. }
                )
            });
            match (has_filter, has_count) {
                (true, true) => "filter_count",
                (true, false) => "filter",
                (false, true) => "count",
                (false, false) => "compose",
            }
        }
        _ => "other",
    }
}

pub(super) fn update_support_actor_hypotheses(
    law: &mut LiveScalarLawState,
    observed: &[ResponseProgram],
) -> Result<(), LiveScalarShadowBlocker> {
    let observed = observed
        .iter()
        .map(|program| {
            serde_cbor::to_vec(program)
                .map(|key| (key, program.clone()))
                .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    if observed.is_empty() || observed.len() > TEACHER_CALL_SELECTOR_BUDGET {
        return Err(LiveScalarShadowBlocker::HypothesisBudgetExhausted);
    }
    if !law.support_hypotheses_initialized {
        law.support_actor_hypotheses = observed.into_values().collect();
        law.support_hypotheses_initialized = true;
        return Ok(());
    }
    let mut union = law
        .support_actor_hypotheses
        .drain(..)
        .map(|program| {
            serde_cbor::to_vec(&program)
                .map(|key| (key, program))
                .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    union.extend(observed);
    if union.len() > TEACHER_CALL_SELECTOR_BUDGET {
        return Err(LiveScalarShadowBlocker::HypothesisBudgetExhausted);
    }
    law.support_actor_hypotheses = union.into_values().collect();
    Ok(())
}

fn live_scalar_support_compiles(law: &LiveScalarLawState) -> bool {
    let Ok(support) = law
        .support
        .iter()
        .map(|transition| {
            reextract_live_scalar_circuit_sample(transition, &law.support_actor_hypotheses)
        })
        .collect::<Result<Vec<_>, _>>()
    else {
        return false;
    };
    let Ok(competing) = build_competing_blueprint_set(&support) else {
        return false;
    };
    FrozenOperatorBlueprintSet::freeze(
        1,
        &competing.support_bundles,
        BlueprintBeamConfig::default(),
        &competing.synthesis,
    )
    .is_ok()
}

fn evaluate_live_law(
    law_key: &str,
    law: &LiveScalarLawState,
    report: &mut LiveScalarShadowReport,
    candidates: &mut Vec<LiveScalarAdmissionCandidate>,
) {
    let (proof_law, censored_support, censored_future) = match provenance_complete_law(law) {
        Ok(result) => result,
        Err(blocker) => {
            increment_report_blocker(report, &blocker);
            return;
        }
    };
    if censored_support != 0 || censored_future != 0 {
        increment_report_blocker(
            report,
            &format!(
                "capture_lineage_censored:support={censored_support}:future={censored_future}"
            ),
        );
    }
    let law = &proof_law;
    let identification = match identify_live_scalar_law_v1(law_key, law) {
        Ok(identification) => identification,
        Err(blocker) => {
            increment_report_blocker(report, &blocker);
            return;
        }
    };
    report.candidate_freezes = report.candidate_freezes.saturating_add(1);
    if law.support_actor_hypotheses.is_empty() {
        increment_report_blocker(report, "actor_hypotheses_no_common_version");
        return;
    }
    let Ok(support) = law
        .support
        .iter()
        .map(|transition| {
            reextract_live_scalar_circuit_sample(transition, &law.support_actor_hypotheses)
        })
        .collect::<Result<Vec<_>, _>>()
    else {
        increment_report_blocker(report, "support_reextract_failed");
        return;
    };
    let competing = match build_competing_blueprint_set(&support) {
        Ok(competing) => competing,
        Err(blocker) => {
            increment_report_blocker(report, &blocker);
            return;
        }
    };
    report.actor_hypotheses = report
        .actor_hypotheses
        .saturating_add(competing.actor_hypothesis_count);
    report.competing_blueprints = report
        .competing_blueprints
        .saturating_add(competing.synthesis.blueprints.len());
    let frozen = match FrozenOperatorBlueprintSet::freeze(
        1,
        &competing.support_bundles,
        BlueprintBeamConfig::default(),
        &competing.synthesis,
    ) {
        Ok(frozen) => frozen,
        Err(error) => {
            increment_report_blocker(
                report,
                &format!("blueprint_freeze_{error:?}").to_lowercase(),
            );
            return;
        }
    };
    report.frozen_laws = report.frozen_laws.saturating_add(1);
    if law.future.is_empty() {
        increment_report_blocker(report, "independent_future_missing");
        return;
    }
    let Some(support_watermark) = law
        .support
        .iter()
        .map(|transition| transition.before.observed_at_unix_nanos)
        .max()
    else {
        increment_report_blocker(report, "support_watermark_missing");
        return;
    };
    // Old fixed-window checkpoints may contain rows labelled as future before
    // a later support replacement. Preserve those rows as history, but never
    // let them become post-freeze transfer authority.
    let transfer_future = law
        .future
        .iter()
        .filter(|transition| transition.before.observed_at_unix_nanos > support_watermark)
        .cloned()
        .collect::<Vec<_>>();
    if transfer_future.is_empty() {
        increment_report_blocker(
            report,
            &format!(
                "independent_future_after_freeze_missing:ignored={}",
                law.future.len()
            ),
        );
        return;
    }
    let Ok(future) = transfer_future
        .iter()
        .map(|transition| {
            reextract_live_scalar_circuit_sample(transition, &law.support_actor_hypotheses)
        })
        .collect::<Result<Vec<_>, _>>()
    else {
        increment_report_blocker(report, "future_reextract_failed");
        return;
    };
    let future_evidence = future
        .iter()
        .map(|sample| {
            // The first seal selects a circuit from structural future only.
            // Binding actor/verifier commitments here would reveal the winner
            // when competing role topologies own different executable laws.
            BlueprintFutureEvidence::new(
                sample.raw_input_sha256,
                sample.extractor_version.max(1),
                sample.bundle.clone(),
            )
        })
        .collect::<Result<Vec<_>, _>>();
    let Ok(future_evidence) = future_evidence else {
        increment_report_blocker(report, "future_evidence_invalid");
        return;
    };
    let full = BlueprintFutureEvaluator::evaluate_and_seal(
        &frozen,
        &future_evidence,
        Default::default(),
        BlueprintPhaseControl::Full,
    );
    let Some(winner) = full.winner_receipt() else {
        let evaluation = full.report();
        let transform_clean = evaluation
            .scores
            .iter()
            .filter(|score| score.transform_mismatches == 0)
            .count();
        let transform_mismatches = evaluation
            .scores
            .iter()
            .map(|score| score.transform_mismatches)
            .sum::<usize>();
        let ambiguous_bindings = evaluation
            .scores
            .iter()
            .map(|score| score.ambiguous_bindings)
            .sum::<usize>();
        let executable_contract_mismatches = evaluation
            .scores
            .iter()
            .map(|score| score.executable_contract_mismatches)
            .sum::<usize>();
        let max_coherence = evaluation
            .scores
            .iter()
            .map(|score| score.whole_circuit_coherence_fixed)
            .max()
            .unwrap_or_default();
        increment_report_blocker(
            report,
            &format!(
                "full_phase_no_winner:{:?}:scores={}:transform_clean={transform_clean}:transform_mismatches={transform_mismatches}:contract_mismatches={executable_contract_mismatches}:ambiguous={ambiguous_bindings}:max_coherence={max_coherence}",
                evaluation.blocker,
                evaluation.scores.len(),
            )
            .to_lowercase(),
        );
        return;
    };
    let Some(actor_template) = competing
        .actors_by_blueprint
        .get(winner.winner_sha256())
        .cloned()
    else {
        increment_report_blocker(report, "winner_actor_contract_missing");
        return;
    };
    // A multi-role template is intentionally unbound. Executing it directly
    // would test support selectors against future surfaces and reject every
    // transferable operator. Crystallization below re-extracts and binds each
    // raw future surface, then independently repeats the binding in verifier.
    let direct_actor_mismatches = if rich_scalar_program_roles(&actor_template)
        .is_some_and(|roles| roles.len() > 1)
        || matches!(
            &actor_template.operation,
            ResponseOperation::FunctionCallFromRoles { .. }
                | ResponseOperation::CustomToolCallFromRoles { .. }
        ) {
        0
    } else {
        future
            .iter()
            .filter(|sample| {
                let Ok(provider_view) = crate::runtime::provider_payload_view(
                    &sample.request_text,
                    &sample.provider_payload,
                ) else {
                    return true;
                };
                execute_response(
                    &actor_template,
                    &sample.request_text,
                    provider_view.as_ref(),
                )
                .response
                .as_deref()
                    != Some(sample.expected_response.as_str())
            })
            .count()
    };
    if direct_actor_mismatches != 0 {
        increment_report_blocker(
            report,
            &format!("winner_actor_future_mismatches={direct_actor_mismatches}"),
        );
        return;
    }
    report.full_phase_winners = report.full_phase_winners.saturating_add(1);
    let controls_pass = [
        BlueprintPhaseControl::NoPhase,
        BlueprintPhaseControl::ShuffledPhase,
        BlueprintPhaseControl::MagnitudeOnly,
        BlueprintPhaseControl::MatchedRandomCenter,
    ]
    .into_iter()
    .all(|control| {
        BlueprintFutureEvaluator::evaluate_and_seal(
            &frozen,
            &future_evidence,
            Default::default(),
            control,
        )
        .winner_receipt()
        .is_none()
    });
    if !controls_pass {
        increment_report_blocker(report, "phase_control_selected_winner");
        return;
    }
    report.causal_control_passes = report.causal_control_passes.saturating_add(1);
    let mut future_window = frozen.future_window();
    for sample in &future {
        if future_window.admit_evidence(&sample.bundle).is_err() {
            increment_report_blocker(report, "future_lineage_rejected");
            return;
        }
    }
    let receipts = future
        .iter()
        .zip(&future_evidence)
        .map(|(sample, evidence)| CrystallizationParityReceipt {
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
        Ok(operator) => {
            report.verified_shadow_operators = report.verified_shadow_operators.saturating_add(1);
            report.shadow_executions = report.shadow_executions.saturating_add(receipts.len());
            match live_admission_candidate(law, &transfer_future, &operator, &identification) {
                Ok(candidate) => {
                    report.transfer_proofs = report.transfer_proofs.saturating_add(1);
                    candidates.push(candidate);
                }
                Err(blocker) => increment_report_blocker(report, &blocker),
            }
        }
        Err(error) => {
            increment_report_blocker(report, &format!("crystallization_{error:?}").to_lowercase())
        }
    }
}

fn provenance_complete_law(
    law: &LiveScalarLawState,
) -> Result<(LiveScalarLawState, usize, usize), String> {
    let has_capture_receipts = law.support.iter().chain(&law.future).any(|transition| {
        transition
            .runtime_parity_case
            .as_ref()
            .and_then(|case| case.capture_receipt.as_ref())
            .is_some()
    });
    if !has_capture_receipts {
        return Ok((law.clone(), 0, 0));
    }

    let mut proof_law = LiveScalarLawState::default();
    for transition in law
        .support
        .iter()
        .filter(|transition| capture_lineage_is_reconstructible(transition))
    {
        let sample = extract_live_scalar_circuit_sample(transition)
            .map_err(|blocker| format!("capture_support_reextract_{blocker:?}").to_lowercase())?;
        update_support_actor_hypotheses(&mut proof_law, &sample.actor_hypotheses)
            .map_err(|blocker| format!("capture_support_hypothesis_{blocker:?}").to_lowercase())?;
        proof_law.support.push(transition.clone());
    }
    proof_law.future = law
        .future
        .iter()
        .filter(|transition| capture_lineage_is_reconstructible(transition))
        .cloned()
        .collect();

    let censored_support = law.support.len().saturating_sub(proof_law.support.len());
    let censored_future = law.future.len().saturating_sub(proof_law.future.len());
    if proof_law.support.is_empty() || proof_law.future.is_empty() {
        return Err(format!(
            "capture_lineage_evidence_empty:support={}:future={}",
            proof_law.support.len(),
            proof_law.future.len()
        ));
    }
    Ok((proof_law, censored_support, censored_future))
}

fn capture_lineage_is_reconstructible(transition: &TeacherTransition) -> bool {
    let Some(parity) = transition.runtime_parity_case.as_ref() else {
        return false;
    };
    let Some(receipt) = parity.capture_receipt.as_ref() else {
        return false;
    };
    let Some(binding) = receipt.transition_binding.as_ref() else {
        return false;
    };
    receipt.validate().is_ok()
        && parity.evidence_ref_sha256 == binding.frame_id_sha256
        && transition
            .verify_capture_frame_id(&binding.frame_id_sha256)
            .is_ok()
}

pub(super) fn build_competing_blueprint_set(
    support: &[LiveScalarCircuitSample],
) -> Result<CompetingBlueprintSet, String> {
    let actors = common_support_actor_hypotheses(support)?;
    let actor_hypothesis_count = actors.len();
    let mut support_bundles = Vec::new();
    let mut blueprints = BTreeMap::new();
    let mut actors_by_blueprint = BTreeMap::new();
    let mut blocker_counts = BTreeMap::new();
    let mut expansions = 0_usize;

    for actor in actors {
        let actor_topology = source_neutral_actor_topology(
            &actor,
            &support[0].request_text,
            &support[0].provider_payload,
        )
        .map(|(topology, _)| topology)
        .ok_or_else(|| "actor_hypothesis_topology_missing".to_owned())?;
        let actor_bundles = support
            .iter()
            .map(|sample| {
                let local_actor = sample
                    .actor_hypotheses
                    .iter()
                    .filter(|program| {
                        source_neutral_actor_topology(
                            program,
                            &sample.request_text,
                            &sample.provider_payload,
                        )
                        .is_some_and(|(topology, _)| topology == actor_topology)
                    })
                    .min_by_key(|program| serde_cbor::to_vec(program).unwrap_or_default())
                    .ok_or_else(|| "actor_support_local_adapter_missing".to_owned())?;
                let roles = rich_scalar_program_roles(local_actor)
                    .ok_or_else(|| "actor_hypothesis_roles_missing".to_owned())?;
                let transform_opcode = program_transform_opcode(local_actor)
                    .ok_or_else(|| "actor_hypothesis_opcode_missing".to_owned())?;
                let transform_flags = program_transform_flags(local_actor)
                    .ok_or_else(|| "actor_hypothesis_flags_missing".to_owned())?;
                observed_rich_scalar_surface(
                    &sample.request_text,
                    &sample.provider_payload,
                    &roles,
                    transform_opcode,
                    transform_flags,
                    program_has_filter_count(local_actor),
                    &commitment_hex(sample.bundle.surface_sha256()),
                    *sample.bundle.lineage_sha256(),
                    *sample.bundle.surface_sha256(),
                )
                .map(|observed| observed.bundle)
                .map_err(|error| format!("actor_support_bundle_{error:?}").to_lowercase())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let available_synthesis_bundles = actor_bundles
            .iter()
            .fold(BTreeMap::new(), |mut lineages, bundle| {
                lineages
                    .entry(*bundle.lineage_sha256())
                    .or_insert_with(|| bundle.clone());
                lineages
            })
            .into_values()
            .take(OPERATOR_BLUEPRINT_MAX_BUNDLES)
            .collect::<Vec<_>>();
        if available_synthesis_bundles.is_empty() {
            return Err("support_sessions_empty".to_owned());
        }
        let mut completed_alignment = false;
        let mut selected = None;
        // A uniquely identified one-role law may crystallize from one support
        // witness; its next independent lineage remains frozen future. Rich or
        // symmetric laws still need more witnesses because alignment or beam
        // synthesis cannot complete while their version space is ambiguous.
        for witness_count in 1..=available_synthesis_bundles.len() {
            let synthesis_bundles = &available_synthesis_bundles[..witness_count];
            let alignments = if witness_count == 1 {
                BoundedRoleAligner::anchor_identified_singleton(
                    &synthesis_bundles[0],
                    RoleAlignmentConfig::default(),
                )
            } else {
                BoundedRoleAligner::align(synthesis_bundles, RoleAlignmentConfig::default())
            };
            if !alignments.completion.is_complete() {
                continue;
            }
            completed_alignment = true;
            let synthesis = BoundedCircuitBeam::synthesize(
                synthesis_bundles,
                &alignments,
                BlueprintBeamConfig::default(),
            );
            if synthesis.completion.is_complete() && !synthesis.blueprints.is_empty() {
                selected = Some((synthesis_bundles.to_vec(), synthesis));
            }
        }
        let Some((synthesis_bundles, synthesis)) = selected else {
            return Err(if completed_alignment {
                "circuit_synthesis_exhausted".to_owned()
            } else {
                "role_alignment_exhausted".to_owned()
            });
        };
        expansions = expansions.saturating_add(synthesis.expansions);
        for blocker in &synthesis.blockers {
            let count = blocker_counts.entry(blocker.blocker).or_insert(0_usize);
            *count = count.saturating_add(blocker.count);
        }

        let verifier = source_neutral_verifier_for_program(&actor)
            .map_err(|error| format!("actor_verifier_build:{error}"))?;
        let actor_sha256 = parse_commitment(
            &response_actor_program_digest(&actor)
                .map_err(|error| format!("actor_digest:{error}"))?,
        )
        .ok_or_else(|| "actor_digest_invalid".to_owned())?;
        let verifier_sha256 = parse_commitment(
            &response_independent_verifier_program_digest(&verifier)
                .map_err(|error| format!("verifier_digest:{error}"))?,
        )
        .ok_or_else(|| "verifier_digest_invalid".to_owned())?;
        for blueprint in synthesis.blueprints {
            let blueprint = blueprint.bind_executable_contracts(actor_sha256, verifier_sha256);
            let fingerprint = *blueprint.fingerprint_sha256();
            if let Some(existing) = actors_by_blueprint.get(&fingerprint) {
                if existing != &actor {
                    return Err("blueprint_actor_commitment_collision".to_owned());
                }
            } else {
                actors_by_blueprint.insert(fingerprint, actor.clone());
            }
            blueprints.entry(fingerprint).or_insert(blueprint);
        }
        // Alternatives from one lineage are committed but never counted as
        // independent evidence; FrozenOperatorBlueprintSet deduplicates lineage.
        support_bundles.extend(synthesis_bundles);
    }

    if blueprints.is_empty() {
        return Err("competing_blueprints_empty".to_owned());
    }
    Ok(CompetingBlueprintSet {
        support_bundles,
        synthesis: BlueprintSynthesisReport {
            blueprints: blueprints
                .into_values()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            expansions,
            completion: SearchCompletion::Complete {
                explored: expansions,
            },
            blockers: blocker_counts
                .into_iter()
                .map(|(blocker, count)| BlueprintSynthesisBlockerCount { blocker, count })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        },
        actors_by_blueprint,
        actor_hypothesis_count,
    })
}

pub(super) fn common_support_actor_hypotheses(
    support: &[LiveScalarCircuitSample],
) -> Result<Vec<ResponseProgram>, String> {
    let Some(first) = support.first() else {
        return Err("actor_hypotheses_missing".to_owned());
    };
    let mut programs_by_topology = BTreeMap::<Vec<u8>, ResponseProgram>::new();
    let mut common_topologies = first
        .actor_hypotheses
        .iter()
        .filter_map(|program| {
            source_neutral_actor_topology(program, &first.request_text, &first.provider_payload)
        })
        .map(|(topology, program)| {
            programs_by_topology
                .entry(topology.clone())
                .or_insert(program);
            topology
        })
        .collect::<BTreeSet<_>>();
    if common_topologies.is_empty() {
        return Err("actor_hypothesis_topology_missing".to_owned());
    }
    for sample in support {
        let mut local_topologies = BTreeSet::new();
        for program in &sample.actor_hypotheses {
            let Some((topology, canonical)) = source_neutral_actor_topology(
                program,
                &sample.request_text,
                &sample.provider_payload,
            ) else {
                continue;
            };
            local_topologies.insert(topology.clone());
            programs_by_topology.entry(topology).or_insert(canonical);
        }
        common_topologies.retain(|topology| local_topologies.contains(topology));
        if common_topologies.is_empty() {
            return Err("actor_hypotheses_no_common_semantic_law".to_owned());
        }
    }
    if common_topologies.len() > COMMON_ACTOR_TOPOLOGY_BUDGET {
        return Err("actor_hypothesis_topology_budget_exhausted".to_owned());
    }
    // Unary physical adapters collapse to one topology. Multi-role role orders
    // remain separate competing blueprints until frozen future resolves them.
    common_topologies
        .into_iter()
        .map(|topology| {
            programs_by_topology
                .remove(&topology)
                .ok_or_else(|| "actor_hypothesis_canonical_program_missing".to_owned())
        })
        .collect()
}

pub(super) fn source_neutral_actor_topology(
    program: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
) -> Option<(Vec<u8>, ResponseProgram)> {
    let role_count = scalar_program_role_slot_types(program)
        .map(|roles| roles.len())
        .or_else(|| rich_scalar_program_roles(program).map(|roles| roles.len()))?;
    let canonical = canonicalize_scalar_program_roles(program, request_text, provider_payload)?;
    if role_count <= 1 {
        return source_neutral_scalar_program_shape(&canonical)
            .map(|shape| ([vec![0], shape].concat(), canonical));
    }
    let encoded = super::induction::source_neutral_multi_role_program_shape(&canonical)?;
    Some(([vec![1], encoded].concat(), canonical))
}

fn live_admission_candidate(
    law: &LiveScalarLawState,
    future: &[TeacherTransition],
    operator: &crate::VerifiedCrystallizedOperator,
    identification: &super::identification::LiveScalarIdentificationV1,
) -> Result<LiveScalarAdmissionCandidate, String> {
    if law.support.is_empty() || future.is_empty() {
        return Err("admission_evidence_empty".to_owned());
    }
    let program = operator
        .routing_program()
        .map_err(|_| "admission_routing_program_failed".to_owned())?;
    let selected_program_root_sha256 =
        nando_operator_kernel::response_program_version_root_sha256(&program)
            .map_err(str::to_owned)?;
    if !identification
        .member_program_roots_sha256
        .contains(&selected_program_root_sha256)
    {
        return Err("admission_routing_program_not_in_frozen_semantic_class".to_owned());
    }
    let verifier = operator
        .routing_verifier()
        .map_err(|_| "admission_routing_verifier_failed".to_owned())?;
    let verifier_schema = crate::response_program_external_verifier_schema(&program)
        .ok_or_else(|| "admission_verifier_schema_missing".to_owned())?;
    let distinct_sessions = law
        .support
        .iter()
        .chain(future)
        .map(|row| row.before.session_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let distinct_surfaces = law
        .support
        .iter()
        .chain(future)
        .map(|row| row.before.frame_id_sha256.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let package_id = format!(
        "crystallized-scalar-{}",
        &commitment_hex(operator.blueprint_sha256())[..16]
    );
    let route_margin = |row: &TeacherTransition| {
        let parity = row.runtime_parity_case.as_ref()?;
        let bound = operator
            .bind_pre_action(&parity.request_text, &parity.provider_payload)
            .ok()?;
        Some(operator.runtime_route_margin(&bound))
    };
    let wave_margin_micro = law
        .support
        .iter()
        .filter_map(&route_margin)
        .min()
        .ok_or_else(|| "admission_circuit_route_missing".to_owned())?;
    if future
        .iter()
        .any(|row| route_margin(row).is_none_or(|margin| margin < wave_margin_micro))
    {
        return Err("admission_circuit_route_future_mismatch".to_owned());
    }
    let package = ResponsePackage {
        schema: "nando.response-package.v1".to_owned(),
        package_id,
        origin: ResponsePackageOrigin::GroundedSynthesis,
        state: ResponsePackageState::Quarantine,
        program,
        verifier: Some(verifier),
        routing_predicates: Vec::new(),
        required_routing_atom_ids: Vec::new(),
        // The legacy vector is retained for package ABI validation. Runtime
        // authority comes from the restored circuit binding below, not from a
        // generic response-program atom masquerading as learned evidence.
        phase_centers: vec![operator.relation_program().fingerprint64()],
        anti_centers: Vec::new(),
        wave_margin_micro,
        learned_wave_route: None,
        crystallized_operator: Some(
            operator
                .restart_bundle()
                .map_err(|_| "admission_restart_bundle_failed".to_owned())?,
        ),
        proof: ResponsePackageProof {
            support_rows: law.support.len(),
            future_rows: future.len(),
            distinct_sessions,
            distinct_surfaces,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: true,
            verifier_schema: verifier_schema.to_owned(),
            adaptive_identification: Some(
                nando_operator_admission::seal_adaptive_identification_proof_v1(
                    nando_operator_admission::AdaptiveIdentificationProofInputV1 {
                        candidate_freeze_root_sha256: identification.freeze_root_sha256.clone(),
                        semantic_class_id_sha256: identification.semantic_class_id_sha256.clone(),
                        // The freeze root still commits the canonical class
                        // representative. This field binds the executable
                        // member selected later by independent future parity.
                        canonical_program_root_sha256: selected_program_root_sha256,
                        applicability_scope_root_sha256: identification
                            .applicability_scope_root_sha256
                            .clone(),
                        transfer_proof_root_sha256: commitment_hex(
                            operator.parity_seal().seal_sha256(),
                        ),
                    },
                )
                .map_err(str::to_owned)?,
            ),
        },
    };
    package
        .validate()
        .map_err(|error| format!("admission_package_{error}"))?;
    let mut candidate = LiveScalarAdmissionCandidate {
        package,
        support: law.support.clone(),
        future: future.to_vec(),
        freeze_watermark_unix_nanos: 0,
        partition_commitment_sha256: String::new(),
        support_root_sha256: commitment_hex(operator.support_root_sha256()),
        future_evidence_root_sha256: commitment_hex(operator.future_evidence_root_sha256()),
        future_lineage_root_sha256: commitment_hex(operator.future_lineage_root_sha256()),
        winner_seal_sha256: commitment_hex(operator.winner_seal_sha256()),
        executable_parity_seal_sha256: commitment_hex(operator.parity_seal().seal_sha256()),
    };
    candidate.seal_evidence_partition().map_err(str::to_owned)?;
    Ok(candidate)
}

fn increment_report_blocker(report: &mut LiveScalarShadowReport, blocker: &str) {
    *report.blockers.entry(blocker.to_owned()).or_default() += 1;
}
