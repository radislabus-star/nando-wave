use super::*;

pub(super) struct AdvanceInput<'a> {
    pub prepared: &'a PreparedK1TickContextV1,
    pub topologies: &'a [PreActionTopologyAuditRowV1],
    pub frames: &'a [RelationFrame],
    pub terminal_receipts: &'a [TransportTerminalReceiptV1],
    pub active_protocol_mode_roots_sha256: &'a BTreeSet<String>,
    pub candidate_artifacts: &'a [NaturalT1ProgramArtifactV1],
    pub generated_at_unix: u64,
}

pub(super) struct PreparedK1TickContextV1 {
    pub join_report: MultiSourceJoinReportV1,
    pub bindings: Vec<EvidenceBinding>,
    pub evidence_epoch_root_sha256: String,
    pub catalog: ValidatedK1NaturalCohortCatalogV1,
    pub motif_archive: Option<MotifEvidenceArchive>,
    pub motif_evidence_epoch_root_sha256: String,
    pub motif_catalog: ValidatedK1NaturalCohortCatalogV1,
    pub active_protocol_mode_set_root_sha256: String,
    pub contract_watermark: u64,
}

#[cfg(test)]
pub(super) fn prepare_tick_context(
    topologies: &[PreActionTopologyAuditRowV1],
    frames: &[RelationFrame],
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
) -> Result<PreparedK1TickContextV1, String> {
    let join_ledger = MultiSourceJoinLedgerV1::build(topologies, frames);
    prepare_tick_context_from_join_ledger(join_ledger, active_protocol_mode_roots_sha256)
}

#[cfg(test)]
pub(super) fn prepare_tick_context_from_join_ledger(
    join_ledger: MultiSourceJoinLedgerV1,
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
) -> Result<PreparedK1TickContextV1, String> {
    let join_report = join_ledger.report();
    let bindings = build_evidence_bindings(join_ledger.into_rows())?;
    prepare_tick_context_from_bindings(join_report, bindings, active_protocol_mode_roots_sha256)
}

pub(super) fn prepare_tick_context_from_bindings(
    join_report: MultiSourceJoinReportV1,
    bindings: Vec<EvidenceBinding>,
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
) -> Result<PreparedK1TickContextV1, String> {
    let motif_archive = build_motif_archive(&bindings)?;
    let (motif_evidence_epoch_root_sha256, motif_catalog) = build_motif_catalog(&motif_archive)?;
    let motif_catalog =
        ValidatedK1NaturalCohortCatalogV1::try_new(motif_catalog).map_err(str::to_owned)?;
    let evidence_epoch_root_sha256 = evidence_epoch_root(&bindings)?;
    let catalog = build_k1_natural_cohort_catalog_v1(
        &bindings
            .iter()
            .map(|binding| binding.row.clone())
            .collect::<Vec<_>>(),
        evidence_epoch_root_sha256.clone(),
        fixture_exclusion_root()?,
        MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V3.to_owned(),
    )
    .map_err(str::to_owned)?;
    let catalog = ValidatedK1NaturalCohortCatalogV1::try_new(catalog).map_err(str::to_owned)?;
    let active_protocol_mode_set_root_sha256 =
        crate::k1_natural_scheduler::duplicate_cohorts::known_epistemic_protocol_mode_set_root(
            active_protocol_mode_roots_sha256,
        )?;
    let contract_watermark = bindings
        .iter()
        .map(|binding| binding.row.capture_sequence)
        .max()
        .unwrap_or(0);

    Ok(PreparedK1TickContextV1 {
        join_report,
        bindings,
        evidence_epoch_root_sha256,
        catalog,
        motif_archive: Some(motif_archive),
        motif_evidence_epoch_root_sha256,
        motif_catalog,
        active_protocol_mode_set_root_sha256,
        contract_watermark,
    })
}

pub(super) fn extend_prepared_tick_context(
    prepared: &mut PreparedK1TickContextV1,
    joined_rows: Vec<BlindThenRevealJoinedTransitionV1>,
    join_report: MultiSourceJoinReportV1,
    active_protocol_mode_roots_sha256: &BTreeSet<String>,
) -> Result<(), String> {
    let previous_binding_rows = prepared.bindings.len();
    extend_evidence_bindings(&mut prepared.bindings, true, joined_rows)?;
    let motif_archive = prepared
        .motif_archive
        .take()
        .ok_or_else(|| "k1_motif_archive_missing".to_owned())?;
    let mut motif_accumulator =
        MotifEvidenceAccumulator::resume(motif_archive, &prepared.bindings)?;
    for (index, binding) in prepared
        .bindings
        .iter()
        .enumerate()
        .skip(previous_binding_rows)
    {
        if !binding.payload_retained() {
            return Err("k1_motif_ambient_payload_missing".to_owned());
        }
        motif_accumulator.push_natural(index, binding.joined())?;
    }
    let motif_archive = motif_accumulator.finish(&prepared.bindings)?;
    let (motif_evidence_epoch_root_sha256, motif_catalog) = build_motif_catalog(&motif_archive)?;
    let motif_catalog =
        ValidatedK1NaturalCohortCatalogV1::try_new(motif_catalog).map_err(str::to_owned)?;
    let evidence_epoch_root_sha256 = evidence_epoch_root(&prepared.bindings)?;
    let catalog = build_k1_natural_cohort_catalog_v1(
        &prepared
            .bindings
            .iter()
            .map(|binding| binding.row.clone())
            .collect::<Vec<_>>(),
        evidence_epoch_root_sha256.clone(),
        fixture_exclusion_root()?,
        MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V3.to_owned(),
    )
    .map_err(str::to_owned)?;
    let catalog = ValidatedK1NaturalCohortCatalogV1::try_new(catalog).map_err(str::to_owned)?;
    let active_protocol_mode_set_root_sha256 =
        crate::k1_natural_scheduler::duplicate_cohorts::known_epistemic_protocol_mode_set_root(
            active_protocol_mode_roots_sha256,
        )?;
    let contract_watermark = prepared
        .bindings
        .iter()
        .map(|binding| binding.row.capture_sequence)
        .max()
        .unwrap_or(0);

    prepared.join_report = join_report;
    prepared.evidence_epoch_root_sha256 = evidence_epoch_root_sha256;
    prepared.catalog = catalog;
    prepared.motif_archive = Some(motif_archive);
    prepared.motif_evidence_epoch_root_sha256 = motif_evidence_epoch_root_sha256;
    prepared.motif_catalog = motif_catalog;
    prepared.active_protocol_mode_set_root_sha256 = active_protocol_mode_set_root_sha256;
    prepared.contract_watermark = contract_watermark;
    Ok(())
}

fn build_motif_archive(bindings: &[EvidenceBinding]) -> Result<MotifEvidenceArchive, String> {
    let mut accumulator = MotifEvidenceAccumulator::new();
    for (index, binding) in bindings.iter().enumerate() {
        if !binding.payload_retained() {
            return Err("k1_motif_ambient_payload_missing".to_owned());
        }
        accumulator.push_natural(index, binding.joined())?;
    }
    accumulator.finish(bindings)
}

fn build_motif_catalog(
    archive: &MotifEvidenceArchive,
) -> Result<(String, K1NaturalCohortCatalogV1), String> {
    let evidence_rows = archive.evidence_rows();
    let evidence_epoch_root_sha256 = canonical_json_sha256(&(
        "nando.k1-motif-evidence-epoch.v1",
        archive.archive_root_sha256.as_str(),
        archive.disposition.summary_root_sha256.as_str(),
        evidence_rows
            .iter()
            .map(|row| row.row_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .map_err(str::to_owned)?;
    let catalog = build_k1_motif_cohort_catalog_v1(
        &evidence_rows,
        &archive.candidate_supports,
        evidence_epoch_root_sha256.clone(),
        fixture_exclusion_root()?,
        MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V4.to_owned(),
        archive.disposition.clone(),
    )
    .map_err(str::to_owned)?;
    Ok((evidence_epoch_root_sha256, catalog))
}

pub(super) fn advance(
    certification: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    allow_candidate_freeze: bool,
    input: AdvanceInput<'_>,
) -> Result<K1NaturalSchedulerRuntimeReportV1, String> {
    let AdvanceInput {
        prepared,
        topologies,
        frames,
        terminal_receipts,
        active_protocol_mode_roots_sha256,
        candidate_artifacts,
        generated_at_unix,
    } = input;
    let join_report = prepared.join_report.clone();
    let bindings = &prepared.bindings;
    let motif_archive = prepared.motif_archive.as_ref();
    let deficit = current_deficit_snapshot(certification)?;
    let active_protocol_mode_set_root_sha256 = &prepared.active_protocol_mode_set_root_sha256;
    let mut projection = restore_projection_for(certification, lane)?;
    let motif_v6 = projection
        .active_candidate_freeze
        .as_ref()
        .is_none_or(|freeze| freeze.schema == K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6);
    let (validated_catalog, expected_evidence_epoch, candidate_freeze_schema, discovery_basis_root) =
        if motif_v6 {
            (
                &prepared.motif_catalog,
                &prepared.motif_evidence_epoch_root_sha256,
                K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6,
                natural_t1_discovery_basis_root_v4().map_err(str::to_owned)?,
            )
        } else {
            (
                &prepared.catalog,
                &prepared.evidence_epoch_root_sha256,
                K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5,
                natural_t1_discovery_basis_root_v3().map_err(str::to_owned)?,
            )
        };
    let catalog = validated_catalog.as_ref().clone();
    if catalog.evidence_epoch_root_sha256 != *expected_evidence_epoch {
        return Err("k1_prepared_context_evidence_epoch_mismatch".to_owned());
    }
    let completed = candidate_exclusions_for(
        certification,
        lane,
        &catalog,
        active_protocol_mode_set_root_sha256,
        candidate_freeze_schema,
        &discovery_basis_root,
    )?;
    let contract_watermark = prepared.contract_watermark;
    let queue = validated_catalog
        .build_candidate_queue_with_exclusions(&deficit, &completed, contract_watermark)
        .map_err(str::to_owned)?;

    if let Some(terminal) = projection.pending_terminal_transfer.as_ref() {
        return runtime_report(
            generated_at_unix,
            lane,
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
        if !allow_candidate_freeze {
            return runtime_report(
                generated_at_unix,
                lane,
                K1NaturalSchedulerRuntimeStateV1::MechanismWatchComplete,
                "mechanism_watch_terminal".to_owned(),
                projection,
                join_report,
                catalog,
                queue,
                None,
                0,
                0,
            );
        }
        if deficit.k1_open {
            return runtime_report(
                generated_at_unix,
                lane,
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
                lane,
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
        let freeze = validated_catalog
            .seal_candidate_freeze(
                projection.next_generation_sequence,
                &deficit,
                &queue,
                &candidate,
                queue_row.score.clone(),
                K1_SCHEDULER_SCHEMA_V2.to_owned(),
                discovery_basis_root,
                generation_budget(),
                candidate.last_capture_sequence,
                contract_watermark,
                generated_at_unix,
            )
            .map_err(str::to_owned)?;
        projection = append_candidate_freeze_for(
            certification,
            lane,
            K1CandidateFreezeAuthorityRequestV1 {
                schema: K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
                lane,
                catalog: catalog.clone(),
                deficit_snapshot: deficit,
                queue: queue.clone(),
                candidate,
                freeze,
                active_protocol_mode_set_root_sha256: active_protocol_mode_set_root_sha256.clone(),
            },
            &projection,
        )?;
        return runtime_report(
            generated_at_unix,
            lane,
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
    let frozen_evidence_rows = frozen_support_count(bindings, motif_archive, &candidate_freeze)?;
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
        projection = append_scheduler_payload_for(
            certification,
            lane,
            K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
        )?;
        return runtime_report(
            generated_at_unix,
            lane,
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
        bindings,
        motif_archive,
        frames,
        active_protocol_mode_roots_sha256,
        candidate_artifacts,
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
            let mut terminal_evidence = vec![
                candidate_freeze.evidence_manifest_root_sha256.clone(),
                base_identification.report_root_sha256.clone(),
            ];
            if blocker == K1_DUPLICATE_PROTOCOL_BLOCKER_V1 {
                terminal_evidence.push(active_protocol_mode_set_root_sha256.clone());
            }
            let verdict = terminal_verdict(
                &candidate_freeze,
                None,
                Vec::new(),
                terminal_evidence,
                K1GenerationVerdictClassV1::AcquisitionFail,
                &blocker,
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
        let prediction_schema = match lane {
            K1SchedulerLaneV1::Mechanism => K1_PREDICTION_SCHEMA_V1,
            K1SchedulerLaneV1::Epistemic => K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1,
        };
        let identification_freeze =
            seal_identification_freeze(&candidate_freeze, &base_identification, prediction_schema)?;
        projection = append_scheduler_payload_for(
            certification,
            lane,
            K1SchedulerEventPayloadV1::IdentificationFreeze(identification_freeze),
        )?;
        return runtime_report(
            generated_at_unix,
            lane,
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
        projection = append_scheduler_payload_for(
            certification,
            lane,
            K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
        )?;
        return runtime_report(
            generated_at_unix,
            lane,
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

    if classes.len() == 1
        && let Some((projection, state, blocker)) = settle_precommitted_future_evidence(
            certification,
            lane,
            &projection,
            &base_identification,
            topologies,
            bindings,
            frames,
            terminal_receipts,
            candidate_artifacts,
        )?
    {
        return runtime_report(
            generated_at_unix,
            lane,
            state,
            blocker.to_owned(),
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
        let deadline = deadline::classify_deadline(
            &classes,
            future_eligible_rows,
            durable_future_prediction_contract(&identification_freeze)
                && projection.future_prediction_contract.is_some(),
            projection
                .future_predictions
                .len()
                .saturating_sub(projection.future_prediction_censors.len()),
            projection
                .future_outcomes
                .iter()
                .filter(|outcome| outcome.independent_verifier_pass)
                .count(),
        );
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
            deadline.verdict,
            deadline.blocker,
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
            deadline.runtime_state,
            deadline.blocker.to_owned(),
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
            lane,
            generated_at_unix,
            projection,
            join_report,
            catalog,
            queue,
            base_identification,
            candidate_freeze,
            identification_freeze,
            bindings,
            motif_archive,
            frames,
            active_protocol_mode_roots_sha256,
            candidate_artifacts,
            applied_roots,
            frozen_evidence_rows,
            future_eligible_rows,
        );
    }

    projection = match advance_future_evidence(
        certification,
        lane,
        projection,
        &candidate_freeze,
        &identification_freeze,
        &base_identification,
        topologies,
        bindings,
        frames,
        terminal_receipts,
        candidate_artifacts,
    )? {
        FutureEvidenceAdvance::Pending {
            projection,
            state,
            blocker,
        } => {
            return runtime_report(
                generated_at_unix,
                lane,
                state,
                blocker.to_owned(),
                projection,
                join_report,
                catalog,
                queue,
                Some(base_identification),
                frozen_evidence_rows,
                future_eligible_rows,
            );
        }
        FutureEvidenceAdvance::Ready(projection) => projection,
    };

    advance_independent_future(
        certification,
        lane,
        generated_at_unix,
        projection,
        join_report,
        catalog,
        queue,
        base_identification,
        candidate_freeze,
        identification_freeze,
        bindings,
        motif_archive,
        frames,
        active_protocol_mode_roots_sha256,
        candidate_artifacts,
        applied_roots,
        frozen_evidence_rows,
        future_eligible_rows,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_tick_context_is_deterministic_and_authority_free() {
        let active_protocols = BTreeSet::new();
        let first = prepare_tick_context(&[], &[], &active_protocols).expect("first context");
        let second = prepare_tick_context(&[], &[], &active_protocols).expect("second context");

        assert_eq!(first.join_report, second.join_report);
        assert_eq!(first.bindings, second.bindings);
        assert_eq!(first.catalog, second.catalog);
        assert_eq!(first.motif_archive, second.motif_archive);
        assert_eq!(first.motif_catalog, second.motif_catalog);
        assert_eq!(
            first.active_protocol_mode_set_root_sha256,
            second.active_protocol_mode_set_root_sha256
        );
        assert_eq!(first.contract_watermark, 0);
        assert_eq!(
            first.evidence_epoch_root_sha256,
            first.catalog.evidence_epoch_root_sha256
        );
        assert_eq!(
            first.motif_evidence_epoch_root_sha256,
            first.motif_catalog.evidence_epoch_root_sha256
        );
        assert!(!first.catalog.authority_ready);
        assert!(!first.motif_catalog.authority_ready);
    }
}
