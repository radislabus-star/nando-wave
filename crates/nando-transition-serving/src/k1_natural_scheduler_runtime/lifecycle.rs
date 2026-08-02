use super::*;

pub(super) fn advance(
    certification: &CertificationAuthorityConfigV1,
    topologies: &[PreActionTopologyAuditRowV1],
    frames: &[RelationFrame],
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
    generated_at_unix: u64,
) -> Result<K1NaturalSchedulerRuntimeReportV1, String> {
    let join_ledger = MultiSourceJoinLedgerV1::build(topologies, frames);
    let join_report = join_ledger.report();
    let bindings = build_evidence_bindings(&join_ledger.rows())?;
    let evidence_epoch_root_sha256 = evidence_epoch_root(&bindings)?;
    let fixture_exclusion_root_sha256 = fixture_exclusion_root()?;
    let catalog = build_k1_natural_cohort_catalog_v1(
        &bindings
            .iter()
            .map(|binding| binding.row.clone())
            .collect::<Vec<_>>(),
        evidence_epoch_root_sha256,
        fixture_exclusion_root_sha256,
        MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V2.to_owned(),
    )
    .map_err(str::to_owned)?;
    let deficit = current_deficit_snapshot(certification)?;
    let mut projection = restore_projection(certification)?;
    let completed = projection
        .completed_candidate_roots_sha256
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let contract_watermark = bindings
        .iter()
        .map(|binding| binding.row.capture_sequence)
        .max()
        .unwrap_or(0);
    let queue = build_k1_natural_candidate_queue_with_exclusions_v1(
        &catalog,
        &deficit,
        &completed,
        contract_watermark,
    )
    .map_err(str::to_owned)?;

    if let Some(terminal) = projection.pending_terminal_transfer.as_ref() {
        return runtime_report(
            generated_at_unix,
            K1NaturalSchedulerRuntimeStateV1::AwaitingCertification,
            "package_certification_pending".to_owned(),
            projection.clone(),
            join_report,
            catalog,
            queue,
            terminal.transfer_identification.clone(),
            0,
            0,
        );
    }

    if projection.active_candidate_freeze.is_none() {
        if deficit.k1_open {
            return runtime_report(
                generated_at_unix,
                K1NaturalSchedulerRuntimeStateV1::K1VocabularyOpen,
                String::new(),
                projection,
                join_report,
                catalog,
                queue,
                None,
                0,
                0,
            );
        }
        let Some(queue_row) = queue.first_readiness_pass() else {
            return runtime_report(
                generated_at_unix,
                K1NaturalSchedulerRuntimeStateV1::WaitingForEvidence,
                "no_readiness_pass_candidate".to_owned(),
                projection,
                join_report,
                catalog,
                queue,
                None,
                0,
                0,
            );
        };
        let candidate = catalog
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_root_sha256 == queue_row.candidate_root_sha256)
            .cloned()
            .ok_or_else(|| "k1_runtime_queue_candidate_missing".to_owned())?;
        let freeze = K1NaturalCandidateFreezeV1::seal(
            projection.next_generation_sequence,
            &catalog,
            &deficit,
            &queue,
            &candidate,
            queue_row.score.clone(),
            K1_SCHEDULER_SCHEMA_V1.to_owned(),
            generation_budget(),
            candidate.last_capture_sequence,
            contract_watermark,
            generated_at_unix,
        )
        .map_err(str::to_owned)?;
        projection = append_candidate_freeze(
            certification,
            K1CandidateFreezeAuthorityRequestV1 {
                schema: K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
                catalog: catalog.clone(),
                deficit_snapshot: deficit,
                queue: queue.clone(),
                candidate,
                freeze,
            },
        )?;
        return runtime_report(
            generated_at_unix,
            K1NaturalSchedulerRuntimeStateV1::CandidateFrozen,
            String::new(),
            projection,
            join_report,
            catalog,
            queue,
            None,
            0,
            0,
        );
    }

    let candidate_freeze = projection
        .active_candidate_freeze
        .clone()
        .ok_or_else(|| "k1_runtime_active_candidate_missing".to_owned())?;
    let support = frozen_support(&bindings, &candidate_freeze)?;
    let frozen_evidence_rows = u64::try_from(support.len()).unwrap_or(u64::MAX);
    let future_eligible_rows = u64::try_from(
        bindings
            .iter()
            .filter(|binding| {
                binding_matches_freeze(binding, &candidate_freeze)
                    && !binding.row.safety_veto
                    && binding.row.capture_sequence >= candidate_freeze.future_min_sequence
            })
            .count(),
    )
    .unwrap_or(u64::MAX);

    if deficit.k1_open {
        let verdict = terminal_verdict(
            &candidate_freeze,
            projection.identification_freeze.as_ref(),
            current_classes(&projection),
            vec![
                candidate_freeze.freeze_root_sha256.clone(),
                deficit.snapshot_root_sha256,
            ],
            K1GenerationVerdictClassV1::Abstain,
            "k1_vocabulary_opened_during_generation",
            generated_at_unix,
            None,
        )?;
        projection = append_scheduler_payload(
            certification,
            K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
        )?;
        return runtime_report(
            generated_at_unix,
            K1NaturalSchedulerRuntimeStateV1::TerminalAbstain,
            "k1_vocabulary_opened_during_generation".to_owned(),
            projection,
            join_report,
            catalog,
            queue,
            None,
            frozen_evidence_rows,
            future_eligible_rows,
        );
    }

    let applied_roots = projection
        .applied_outcome_roots_sha256
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let base_identification = identify_frozen_candidate(
        &bindings,
        frames,
        active_protocol_mode_roots_sha256,
        &candidate_freeze,
        &applied_roots,
        &BTreeSet::new(),
    )?;

    if projection.identification_freeze.is_none() {
        if !identification_can_freeze(&base_identification) {
            let blocker = base_identification
                .blocker
                .clone()
                .unwrap_or_else(|| "identification_did_not_produce_version_space".to_owned());
            let verdict = terminal_verdict(
                &candidate_freeze,
                None,
                Vec::new(),
                vec![
                    candidate_freeze.evidence_manifest_root_sha256.clone(),
                    base_identification.report_root_sha256.clone(),
                ],
                K1GenerationVerdictClassV1::AcquisitionFail,
                &blocker,
                generated_at_unix,
                None,
            )?;
            projection = append_scheduler_payload(
                certification,
                K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
            )?;
            return runtime_report(
                generated_at_unix,
                K1NaturalSchedulerRuntimeStateV1::TerminalAcquisitionFail,
                blocker,
                projection,
                join_report,
                catalog,
                queue,
                Some(base_identification),
                frozen_evidence_rows,
                future_eligible_rows,
            );
        }
        let identification_freeze =
            seal_identification_freeze(&candidate_freeze, &base_identification)?;
        projection = append_scheduler_payload(
            certification,
            K1SchedulerEventPayloadV1::IdentificationFreeze(identification_freeze),
        )?;
        return runtime_report(
            generated_at_unix,
            K1NaturalSchedulerRuntimeStateV1::IdentificationFrozen,
            String::new(),
            projection,
            join_report,
            catalog,
            queue,
            Some(base_identification),
            frozen_evidence_rows,
            future_eligible_rows,
        );
    }

    let identification_freeze = projection
        .identification_freeze
        .clone()
        .ok_or_else(|| "k1_runtime_identification_freeze_missing".to_owned())?;
    let classes = current_classes(&projection);
    if classes.is_empty() || base_identification.remaining_semantic_class_roots_sha256 != classes {
        let verdict = terminal_verdict(
            &candidate_freeze,
            Some(&identification_freeze),
            classes.clone(),
            vec![
                base_identification.report_root_sha256.clone(),
                identification_freeze.freeze_root_sha256.clone(),
            ],
            K1GenerationVerdictClassV1::Abstain,
            "durable_version_space_replay_mismatch",
            generated_at_unix,
            None,
        )?;
        projection = append_scheduler_payload(
            certification,
            K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
        )?;
        return runtime_report(
            generated_at_unix,
            K1NaturalSchedulerRuntimeStateV1::TerminalAbstain,
            "durable_version_space_replay_mismatch".to_owned(),
            projection,
            join_report,
            catalog,
            queue,
            Some(base_identification),
            frozen_evidence_rows,
            future_eligible_rows,
        );
    }

    if generation_expired(&candidate_freeze, generated_at_unix) {
        let verdict = terminal_verdict(
            &candidate_freeze,
            Some(&identification_freeze),
            classes,
            vec![
                candidate_freeze.freeze_root_sha256.clone(),
                projection.latest_probe_round.as_ref().map_or_else(
                    || identification_freeze.freeze_root_sha256.clone(),
                    |receipt| receipt.receipt_root_sha256.clone(),
                ),
            ],
            K1GenerationVerdictClassV1::ProbeExhausted,
            "generation_deadline_exhausted",
            generated_at_unix,
            None,
        )?;
        projection = append_scheduler_payload(
            certification,
            K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
        )?;
        return runtime_report(
            generated_at_unix,
            K1NaturalSchedulerRuntimeStateV1::TerminalProbeExhausted,
            "generation_deadline_exhausted".to_owned(),
            projection,
            join_report,
            catalog,
            queue,
            Some(base_identification),
            frozen_evidence_rows,
            future_eligible_rows,
        );
    }

    if classes.len() > 1 {
        return advance_probe(
            certification,
            generated_at_unix,
            projection,
            join_report,
            catalog,
            queue,
            base_identification,
            candidate_freeze,
            identification_freeze,
            &bindings,
            frames,
            active_protocol_mode_roots_sha256,
            applied_roots,
            frozen_evidence_rows,
            future_eligible_rows,
        );
    }

    advance_independent_future(
        certification,
        generated_at_unix,
        projection,
        join_report,
        catalog,
        queue,
        base_identification,
        candidate_freeze,
        identification_freeze,
        &bindings,
        frames,
        active_protocol_mode_roots_sha256,
        applied_roots,
        frozen_evidence_rows,
        future_eligible_rows,
    )
}
