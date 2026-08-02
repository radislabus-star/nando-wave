use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::k1_natural_scheduler_runtime) fn advance_independent_future(
    certification: &CertificationAuthorityConfigV1,
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
    if !durable_future_prediction_contract(&identification_freeze) {
        return runtime_report(
            generated_at_unix,
            K1NaturalSchedulerRuntimeStateV1::AwaitingIndependentFuture,
            "independent_future_prediction_contract_missing".to_owned(),
            projection,
            join,
            catalog,
            queue,
            Some(base_identification),
            frozen_evidence_rows,
            future_eligible_rows,
        );
    }
    let consumed = projection
        .consumed_outcome_roots_sha256
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let used_lineages = bindings
        .iter()
        .filter(|binding| {
            binding_matches_freeze(binding, &candidate_freeze)
                && !binding.row.safety_veto
                && (binding.row.capture_sequence <= candidate_freeze.contract_watermark
                    || applied_roots.contains(&binding.joined.join_root_sha256))
        })
        .map(|binding| binding.row.lineage_root_sha256.clone())
        .collect::<BTreeSet<_>>();
    let Some(future) = next_future_binding(
        bindings,
        &candidate_freeze,
        &consumed,
        candidate_freeze.future_min_sequence,
        Some(&used_lineages),
    ) else {
        return runtime_report(
            generated_at_unix,
            K1NaturalSchedulerRuntimeStateV1::AwaitingIndependentFuture,
            "independent_post_identification_future_pending".to_owned(),
            projection,
            join,
            catalog,
            queue,
            Some(base_identification),
            frozen_evidence_rows,
            future_eligible_rows,
        );
    };
    let future_roots = bindings
        .iter()
        .filter(|binding| {
            binding_matches_freeze(binding, &candidate_freeze)
                && !binding.row.safety_veto
                && binding.row.capture_sequence >= candidate_freeze.future_min_sequence
                && binding.row.capture_sequence <= future.row.capture_sequence
                && !consumed.contains(&binding.joined.join_root_sha256)
        })
        .map(|binding| binding.joined.join_root_sha256.clone())
        .collect::<BTreeSet<_>>();
    let trial = identify_frozen_candidate(
        bindings,
        frames,
        active_protocol_mode_roots_sha256,
        &candidate_freeze,
        &applied_roots,
        &future_roots,
    )?;
    let future_frame_id = frames
        .iter()
        .find(|frame| {
            canonical_json_sha256(*frame)
                .is_ok_and(|root| root == future.joined.completed_frame_root_sha256)
        })
        .map(|frame| frame.frame_id_sha256.as_str());
    let future_in_basis = future_frame_id.is_some_and(|frame_id| {
        trial.proof_basis.as_ref().is_some_and(|basis| {
            basis
                .future_capture_frame_ids_sha256
                .iter()
                .any(|root| root == frame_id)
        })
    });
    let protocol_stable = trial.selected_protocol_mode_root_sha256
        == base_identification.selected_protocol_mode_root_sha256;
    let (verdict_class, runtime_state, blocker) = match trial.state {
        MultiSourceT1IdentificationStateV1::TransferReady if future_in_basis && protocol_stable => {
            (
                K1GenerationVerdictClassV1::Pass,
                K1NaturalSchedulerRuntimeStateV1::TerminalPass,
                String::new(),
            )
        }
        MultiSourceT1IdentificationStateV1::FutureContradiction => (
            K1GenerationVerdictClassV1::Abstain,
            K1NaturalSchedulerRuntimeStateV1::TerminalAbstain,
            "independent_future_contradiction".to_owned(),
        ),
        _ if !protocol_stable => (
            K1GenerationVerdictClassV1::Abstain,
            K1NaturalSchedulerRuntimeStateV1::TerminalAbstain,
            "independent_future_protocol_rebound".to_owned(),
        ),
        _ => {
            return runtime_report(
                generated_at_unix,
                K1NaturalSchedulerRuntimeStateV1::AwaitingIndependentFuture,
                "independent_future_not_yet_transfer_complete".to_owned(),
                projection,
                join,
                catalog,
                queue,
                Some(trial),
                frozen_evidence_rows,
                future_eligible_rows,
            );
        }
    };
    let mut evidence = vec![
        trial.report_root_sha256.clone(),
        future.joined.join_root_sha256.clone(),
    ];
    if let Some(basis) = &trial.proof_basis {
        evidence.push(basis.basis_root_sha256.clone());
    }
    let verdict = terminal_verdict(
        &candidate_freeze,
        Some(&identification_freeze),
        current_classes(&projection),
        evidence,
        verdict_class,
        &blocker,
        generated_at_unix,
        (verdict_class == K1GenerationVerdictClassV1::Pass).then(|| trial.clone()),
    )?;
    projection = append_scheduler_payload(
        certification,
        K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
    )?;
    runtime_report(
        generated_at_unix,
        runtime_state,
        blocker,
        projection,
        join,
        catalog,
        queue,
        Some(trial),
        frozen_evidence_rows,
        future_eligible_rows,
    )
}
