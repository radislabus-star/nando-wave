use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProbeOutcomeDisposition {
    Applied,
    NoInformation,
    Contradiction(&'static str),
}

#[allow(clippy::too_many_arguments)]
pub(in crate::k1_natural_scheduler_runtime) fn advance_probe(
    certification: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    generated_at_unix: u64,
    mut projection: K1SchedulerProjectionV1,
    join: MultiSourceJoinReportV1,
    catalog: K1NaturalCohortCatalogV1,
    queue: K1NaturalCandidateQueueV1,
    base_identification: MultiSourceT1IdentificationV3,
    candidate_freeze: K1NaturalCandidateFreezeV1,
    identification_freeze: K1IdentificationFreezeV1,
    bindings: &[EvidenceBinding],
    frames: &[RelationFrame],
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
    applied_roots: BTreeSet<String>,
    frozen_evidence_rows: u64,
    future_eligible_rows: u64,
) -> Result<K1NaturalSchedulerRuntimeReportV1, String> {
    let classes = current_classes(&projection);
    if projection
        .latest_probe_round
        .as_ref()
        .is_some_and(|receipt| receipt.state == K1ProbeRoundStateV1::ProbePending)
    {
        let pending = projection
            .latest_probe_round
            .clone()
            .ok_or_else(|| "k1_runtime_pending_probe_missing".to_owned())?;
        let probe = base_identification
            .passive_probe
            .as_ref()
            .ok_or_else(|| "k1_runtime_pending_probe_reconstruction_missing".to_owned())?;
        validate_pending_probe(&pending, probe)?;
        let consumed = projection
            .consumed_outcome_roots_sha256
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let Some(future) = next_future_binding(
            bindings,
            &candidate_freeze,
            &consumed,
            pending.outcome_min_capture_sequence,
            None,
        ) else {
            return runtime_report(
                generated_at_unix,
                lane,
                K1NaturalSchedulerRuntimeStateV1::ProbePending,
                "eligible_probe_outcome_pending".to_owned(),
                projection,
                join,
                catalog,
                queue,
                Some(base_identification),
                frozen_evidence_rows,
                future_eligible_rows,
            );
        };
        let mut trial_roots = BTreeSet::new();
        trial_roots.insert(future.joined.join_root_sha256.clone());
        let trial = identify_frozen_candidate(
            bindings,
            frames,
            active_protocol_mode_roots_sha256,
            &candidate_freeze,
            &applied_roots,
            &trial_roots,
        )?;
        let next_classes = trial.remaining_semantic_class_roots_sha256.clone();
        let protocol_stable = trial.selected_protocol_mode_root_sha256
            == base_identification.selected_protocol_mode_root_sha256;
        let partition_precommitted = predicted_partition(probe, &next_classes);
        let disposition = classify_probe_outcome(
            &classes,
            &next_classes,
            trial.state,
            protocol_stable,
            partition_precommitted,
        );
        let applied = disposition == ProbeOutcomeDisposition::Applied;
        let outcome = K1ProbeRoundReceiptV1::seal_outcome(
            &pending,
            future.joined.join_root_sha256.clone(),
            future.joined.verifier_receipt_root_sha256.clone(),
            if applied {
                next_classes
            } else {
                classes.clone()
            },
            !applied,
        )
        .map_err(str::to_owned)?;
        let outcome_root = outcome.receipt_root_sha256.clone();
        projection = append_scheduler_payload_for(
            certification,
            lane,
            K1SchedulerEventPayloadV1::ProbeRound(outcome),
        )?;
        if let ProbeOutcomeDisposition::Contradiction(blocker) = disposition {
            let verdict = terminal_verdict(
                &candidate_freeze,
                Some(&identification_freeze),
                current_classes(&projection),
                vec![
                    trial.report_root_sha256.clone(),
                    future.joined.join_root_sha256.clone(),
                    future.joined.verifier_receipt_root_sha256.clone(),
                    outcome_root,
                ],
                K1GenerationVerdictClassV1::Abstain,
                blocker,
                generated_at_unix,
                None,
            )?;
            projection = append_scheduler_payload_for(
                certification,
                lane,
                K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
            )?;
            return runtime_report(
                generated_at_unix,
                lane,
                K1NaturalSchedulerRuntimeStateV1::TerminalAbstain,
                blocker.to_owned(),
                projection,
                join,
                catalog,
                queue,
                Some(trial),
                frozen_evidence_rows,
                future_eligible_rows,
            );
        }
        let state = if applied {
            K1NaturalSchedulerRuntimeStateV1::ProbeOutcomeApplied
        } else {
            K1NaturalSchedulerRuntimeStateV1::ProbeOutcomeCensored
        };
        return runtime_report(
            generated_at_unix,
            lane,
            state,
            String::new(),
            projection,
            join,
            catalog,
            queue,
            Some(trial),
            frozen_evidence_rows,
            future_eligible_rows,
        );
    }

    let probe = base_identification.passive_probe.as_ref();
    let remaining = projection
        .remaining_probe_budget
        .ok_or_else(|| "k1_runtime_probe_budget_missing".to_owned())?;
    let exhausted = probe.is_none()
        || remaining.probe_rounds == 0
        || probe.is_some_and(|value| value.estimated_cost_units > remaining.probe_cost_units);
    if exhausted {
        let verdict = terminal_verdict(
            &candidate_freeze,
            Some(&identification_freeze),
            classes,
            vec![
                base_identification.report_root_sha256.clone(),
                identification_freeze.freeze_root_sha256.clone(),
            ],
            K1GenerationVerdictClassV1::ProbeExhausted,
            "distinguishing_probe_budget_exhausted",
            generated_at_unix,
            None,
        )?;
        projection = append_scheduler_payload_for(
            certification,
            lane,
            K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
        )?;
        return runtime_report(
            generated_at_unix,
            lane,
            K1NaturalSchedulerRuntimeStateV1::TerminalProbeExhausted,
            "distinguishing_probe_budget_exhausted".to_owned(),
            projection,
            join,
            catalog,
            queue,
            Some(base_identification),
            frozen_evidence_rows,
            future_eligible_rows,
        );
    }
    let probe = probe.ok_or_else(|| "k1_runtime_probe_missing".to_owned())?;
    let predictions = k1_predictions(probe);
    let pending = K1ProbeRoundReceiptV1::seal_pending(
        identification_freeze.freeze_root_sha256,
        projection.completed_probe_rounds.saturating_add(1),
        classes,
        probe.probe_root_sha256.clone(),
        probe.observable_difference_root_sha256.clone(),
        probe.precommitted_predictions_root_sha256.clone(),
        predictions,
        bindings
            .iter()
            .map(|binding| binding.row.capture_sequence)
            .max()
            .unwrap_or(candidate_freeze.contract_watermark)
            .saturating_add(1),
        K1ProbeBudgetRemainingV1 {
            probe_rounds: remaining.probe_rounds.saturating_sub(1),
            probe_cost_units: remaining
                .probe_cost_units
                .saturating_sub(probe.estimated_cost_units),
        },
    )
    .map_err(str::to_owned)?;
    projection = append_scheduler_payload_for(
        certification,
        lane,
        K1SchedulerEventPayloadV1::ProbeRound(pending),
    )?;
    runtime_report(
        generated_at_unix,
        lane,
        K1NaturalSchedulerRuntimeStateV1::ProbePending,
        String::new(),
        projection,
        join,
        catalog,
        queue,
        Some(base_identification),
        frozen_evidence_rows,
        future_eligible_rows,
    )
}

fn classify_probe_outcome(
    previous_classes: &[String],
    next_classes: &[String],
    trial_state: MultiSourceT1IdentificationStateV1,
    protocol_stable: bool,
    partition_precommitted: bool,
) -> ProbeOutcomeDisposition {
    if trial_state == MultiSourceT1IdentificationStateV1::FutureContradiction {
        return ProbeOutcomeDisposition::Contradiction("probe_future_contradiction");
    }
    if !protocol_stable {
        return ProbeOutcomeDisposition::Contradiction("probe_protocol_rebound");
    }
    let previous = previous_classes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let next = next_classes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if next == previous {
        return ProbeOutcomeDisposition::NoInformation;
    }
    if next.is_empty() {
        return ProbeOutcomeDisposition::Contradiction("probe_version_space_exhausted");
    }
    if !next.is_subset(&previous) {
        return ProbeOutcomeDisposition::Contradiction("probe_version_space_expanded");
    }
    if !partition_precommitted {
        return ProbeOutcomeDisposition::Contradiction("probe_outcome_not_precommitted");
    }
    ProbeOutcomeDisposition::Applied
}

#[cfg(test)]
mod tests;
