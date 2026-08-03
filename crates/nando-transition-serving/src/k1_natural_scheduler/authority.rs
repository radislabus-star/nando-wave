use super::future_authority::*;
use super::journal::*;
use super::projection::projection_for;
use super::selection_authority::validate_queue_derivation;
use super::*;

pub(crate) fn handle_authority_line(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    line: &str,
) -> Option<String> {
    let value: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => return None,
    };
    let schema = value.get("schema").and_then(Value::as_str)?;
    let result = match schema {
        K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1 => serde_json::from_value::<
            K1CandidateFreezeAuthorityRequestV1,
        >(value)
        .map_err(|error| format!("k1_candidate_freeze_request_decode:{error}"))
        .and_then(|request| append_candidate_freeze_authoritative(config, signing_key, request)),
        K1_SCHEDULER_APPEND_AUTHORITY_REQUEST_SCHEMA_V1 => {
            serde_json::from_value::<K1SchedulerAppendAuthorityRequestV1>(value)
                .map_err(|error| format!("k1_scheduler_append_request_decode:{error}"))
                .and_then(|request| append_payload_authoritative(config, signing_key, request))
        }
        K1_TRANSFER_SETTLEMENT_AUTHORITY_REQUEST_SCHEMA_V1 => serde_json::from_value::<
            K1TransferSettlementAuthorityRequestV1,
        >(value)
        .map_err(|error| format!("k1_transfer_settlement_request_decode:{error}"))
        .and_then(|request| append_transfer_settlement_authoritative(config, signing_key, request)),
        K1_FUTURE_CONTRACT_AUTHORITY_REQUEST_SCHEMA_V1 => serde_json::from_value::<
            K1FutureContractAuthorityRequestV1,
        >(value)
        .map_err(|error| format!("k1_future_contract_request_decode:{error}"))
        .and_then(|request| append_future_contract_authoritative(config, signing_key, request)),
        K1_FUTURE_PREDICTION_AUTHORITY_REQUEST_SCHEMA_V1 => serde_json::from_value::<
            K1FuturePredictionAuthorityRequestV1,
        >(value)
        .map_err(|error| format!("k1_future_prediction_request_decode:{error}"))
        .and_then(|request| append_future_prediction_authoritative(config, signing_key, request)),
        K1_PRE_ACTION_EVIDENCE_AUTHORITY_REQUEST_SCHEMA_V1 => {
            serde_json::from_value::<K1PreActionEvidenceAuthorityRequestV1>(value)
                .map_err(|error| format!("k1_pre_action_evidence_request_decode:{error}"))
                .and_then(|request| archive_pre_action_evidence_authoritative(config, request))
        }
        K1_FUTURE_OUTCOME_AUTHORITY_REQUEST_SCHEMA_V1 => serde_json::from_value::<
            K1FutureOutcomeAuthorityRequestV1,
        >(value)
        .map_err(|error| format!("k1_future_outcome_request_decode:{error}"))
        .and_then(|request| append_future_outcome_authoritative(config, signing_key, request)),
        _ => return None,
    };
    let response = match result {
        Ok(projection) => K1SchedulerAuthorityResponseV1 {
            schema: K1_SCHEDULER_AUTHORITY_RESPONSE_SCHEMA_V1.to_owned(),
            projection: Some(projection),
            error: String::new(),
        },
        Err(error) => K1SchedulerAuthorityResponseV1 {
            schema: K1_SCHEDULER_AUTHORITY_RESPONSE_SCHEMA_V1.to_owned(),
            projection: None,
            error,
        },
    };
    Some(serde_json::to_string(&response).unwrap_or_else(|error| {
        format!(
            "{{\"schema\":\"{K1_SCHEDULER_AUTHORITY_RESPONSE_SCHEMA_V1}\",\"projection\":null,\"error\":\"k1_scheduler_response_encode:{error}\"}}"
        )
    }))
}

pub(crate) fn recover_authority(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
) -> Result<(), String> {
    recover_lane(config, signing_key, K1SchedulerLaneV1::Mechanism)?;
    let mechanism = restore_projection_for(config, K1SchedulerLaneV1::Mechanism)?;
    if mechanism.active_candidate_freeze.is_some() && mechanism.identification_freeze.is_some() {
        fork::ensure_epistemic_lane(config, signing_key)?;
        recover_lane(config, signing_key, K1SchedulerLaneV1::Epistemic)?;
    }
    Ok(())
}

fn recover_lane(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    lane: K1SchedulerLaneV1,
) -> Result<(), String> {
    let (ledger, last_event_root) = restore_signed_scheduler_journal_for(config, lane)?;
    let anchor_path = scheduler_anchor_path_for(config, lane)?;
    if !anchor_path.exists() {
        if ledger.revision != 0 {
            return Err("k1_scheduler_anchor_missing_for_nonempty_journal".to_owned());
        }
        return persist_scheduler_anchor_for(config, lane, signing_key, &ledger, &last_event_root);
    }
    let anchor = restore_scheduler_anchor_for(config, lane)?;
    if anchor.revision == ledger.revision
        && anchor.journal_event_root_sha256 == last_event_root
        && anchor.ledger_root_sha256 == ledger.ledger_root_sha256
    {
        return Ok(());
    }
    if anchor.revision >= ledger.revision {
        return Err("k1_scheduler_rollback_detected".to_owned());
    }
    let (prefix, prefix_event_root) =
        restore_scheduler_journal_prefix_for(config, lane, anchor.revision)?;
    if prefix.ledger_root_sha256 != anchor.ledger_root_sha256
        || prefix_event_root != anchor.journal_event_root_sha256
    {
        return Err("k1_scheduler_rollback_detected".to_owned());
    }
    persist_scheduler_anchor_for(config, lane, signing_key, &ledger, &last_event_root)
}

fn append_candidate_freeze_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1CandidateFreezeAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_CANDIDATE_FREEZE_AUTHORITY_REQUEST_SCHEMA_V1 {
        return Err("k1_candidate_freeze_request_schema_invalid".to_owned());
    }
    request.catalog.validate().map_err(str::to_owned)?;
    request.deficit_snapshot.validate().map_err(str::to_owned)?;
    request.queue.validate().map_err(str::to_owned)?;
    request.candidate.validate().map_err(str::to_owned)?;
    request.freeze.validate().map_err(str::to_owned)?;
    validate_active_protocol_mode_cas(config, &request.active_protocol_mode_set_root_sha256)?;

    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    let mut completed_candidate_roots_sha256 = projection_for(&scheduler)?
        .completed_candidate_roots_sha256
        .into_iter()
        .collect::<BTreeSet<_>>();
    if request.lane == K1SchedulerLaneV1::Epistemic {
        completed_candidate_roots_sha256.extend(fork::epistemic_exclusions(config)?);
        completed_candidate_roots_sha256.extend(duplicate_cohorts::duplicate_candidate_exclusions(
            &scheduler,
            &request.catalog,
            &request.active_protocol_mode_set_root_sha256,
        )?);
    }
    validate_queue_derivation(
        &request.catalog,
        &request.deficit_snapshot,
        &completed_candidate_roots_sha256,
        request.freeze.contract_watermark,
        &request.queue,
    )?;
    if scheduler
        .active_candidate_freeze()
        .is_some_and(|freeze| freeze == &request.freeze)
    {
        return projection_for(&scheduler);
    }
    let registry = restore_anchored_ledger(config)?;
    validate_registry_cas(&registry, &request.deficit_snapshot)?;
    let expected = K1NaturalCandidateFreezeV1::seal(
        request.freeze.generation_sequence,
        &request.catalog,
        &request.deficit_snapshot,
        &request.queue,
        &request.candidate,
        request.freeze.scoring_tuple.clone(),
        request.freeze.scheduler_schema.clone(),
        request.freeze.budget,
        request.freeze.support_watermark,
        request.freeze.contract_watermark,
        request.freeze.selected_at_unix,
    )
    .map_err(str::to_owned)?;
    if expected != request.freeze {
        return Err("k1_candidate_freeze_reseal_mismatch".to_owned());
    }
    append_and_persist(
        config,
        request.lane,
        signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::CandidateFreeze(request.freeze),
    )
}

pub(super) fn validate_active_protocol_mode_cas(
    config: &CertificationAuthorityConfigV1,
    claimed_root_sha256: &str,
) -> Result<(), String> {
    let current_active_modes =
        crate::multi_source_live::active_protocol_mode_roots(&config.response_registry_path)?;
    let current_root = duplicate_cohorts::active_protocol_mode_set_root(&current_active_modes)?;
    if claimed_root_sha256 != current_root {
        return Err("k1_candidate_freeze_active_protocol_mode_cas_failed".to_owned());
    }
    Ok(())
}

fn append_payload_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1SchedulerAppendAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_SCHEDULER_APPEND_AUTHORITY_REQUEST_SCHEMA_V1
        || matches!(
            request.payload,
            K1SchedulerEventPayloadV1::CandidateFreeze(_)
                | K1SchedulerEventPayloadV1::TransferSettlement(_)
                | K1SchedulerEventPayloadV1::FuturePredictionContract(_)
                | K1SchedulerEventPayloadV1::FuturePrediction(_)
                | K1SchedulerEventPayloadV1::FutureOutcome(_)
        )
    {
        return Err("k1_scheduler_append_request_invalid".to_owned());
    }
    request.payload.validate().map_err(str::to_owned)?;
    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    if scheduler
        .latest_event()
        .is_some_and(|event| payload_root(&event.payload) == payload_root(&request.payload))
    {
        return projection_for(&scheduler);
    }
    append_and_persist(
        config,
        request.lane,
        signing_key,
        &mut scheduler,
        request.payload,
    )
}

fn append_transfer_settlement_authoritative(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    request: K1TransferSettlementAuthorityRequestV1,
) -> Result<K1SchedulerProjectionV1, String> {
    if request.schema != K1_TRANSFER_SETTLEMENT_AUTHORITY_REQUEST_SCHEMA_V1 {
        return Err("k1_transfer_settlement_request_schema_invalid".to_owned());
    }
    request.settlement.validate().map_err(str::to_owned)?;
    let certification = restore_anchored_ledger(config)?;
    if certification.ledger_root_sha256 != request.settlement.certification_ledger_root_sha256 {
        return Err("k1_transfer_settlement_certification_cas_failed".to_owned());
    }
    let entry = certification
        .latest_entries()
        .into_iter()
        .find(|entry| entry.package_id == request.settlement.package_id)
        .ok_or_else(|| "k1_transfer_settlement_certification_missing".to_owned())?;
    if !certification_authorizes_settlement(entry, &request.settlement) {
        return Err("k1_transfer_settlement_certification_invalid".to_owned());
    }
    let mut scheduler = restore_anchored_scheduler_for(config, request.lane)?;
    if scheduler.latest_event().is_some_and(|event| {
        matches!(
            &event.payload,
            K1SchedulerEventPayloadV1::TransferSettlement(existing)
                if existing == &request.settlement
        )
    }) {
        return projection_for(&scheduler);
    }
    append_and_persist(
        config,
        request.lane,
        signing_key,
        &mut scheduler,
        K1SchedulerEventPayloadV1::TransferSettlement(request.settlement),
    )
}

pub(super) fn certification_authorizes_settlement(
    entry: &nando_operator_admission::OperatorCertificationEntryV1,
    settlement: &K1TransferSettlementV1,
) -> bool {
    entry.package_id == settlement.package_id
        && entry.entry_root_sha256 == settlement.certification_entry_root_sha256
        && entry.law.certificate_root_sha256 == settlement.law_certificate_root_sha256
        && entry.law.status == nando_operator_admission::LawCertificateStatusV1::Pass
        && entry.product_registry_member
        && entry.epistemic_registry_member
        && entry.k1_unit_eligible
        && entry.false_bad_apply == 0
        && entry
            .law
            .evidence_roots_sha256
            .contains(&settlement.package_candidate_root_sha256)
        && entry
            .law
            .evidence_roots_sha256
            .contains(&settlement.terminal_verdict_root_sha256)
        && entry
            .law
            .evidence_roots_sha256
            .contains(&settlement.identification_report_root_sha256)
}

pub(super) fn append_and_persist(
    config: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    signing_key: &SigningKey,
    ledger: &mut K1SchedulerLedgerV1,
    payload: K1SchedulerEventPayloadV1,
) -> Result<K1SchedulerProjectionV1, String> {
    let event = ledger.append(payload).map_err(str::to_owned)?.clone();
    let signed =
        SignedSchedulerEventV1::seal(event, ledger.ledger_root_sha256.clone(), signing_key)?;
    persist_scheduler_event_for(config, lane, &signed)?;
    persist_scheduler_anchor_for(
        config,
        lane,
        signing_key,
        ledger,
        &signed.event.event_root_sha256,
    )?;
    persist_scheduler_cache_for(config, lane, ledger)?;
    let restored = restore_anchored_scheduler_for(config, lane)?;
    if &restored != ledger {
        return Err("k1_scheduler_restart_parity_failed".to_owned());
    }
    projection_for(ledger)
}

pub(super) fn validate_registry_cas(
    ledger: &nando_operator_admission::OperatorCertificationLedgerV1,
    snapshot: &K1DeficitSnapshotV1,
) -> Result<(), String> {
    let gate = ledger.k1_vocabulary_gate().map_err(str::to_owned)?;
    let eligible = ledger
        .latest_entries()
        .into_iter()
        .filter(|entry| entry.k1_unit_eligible)
        .collect::<Vec<_>>();
    let semantic_roots = eligible
        .iter()
        .map(|entry| entry.semantic_law_id_sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let topology_roots = eligible
        .iter()
        .map(|entry| entry.role_topology_id_sha256.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if snapshot.epistemic_registry_revision != ledger.revision
        || snapshot.epistemic_registry_root_sha256 != ledger.ledger_root_sha256
        || snapshot.k1_gate_root_sha256 != gate.gate_root_sha256
        || snapshot.law_certificates != gate.law_certificates
        || snapshot.semantic_laws != gate.semantic_laws
        || snapshot.role_topologies != gate.role_topologies
        || snapshot.cleanup_receipts != gate.cleanup_receipts
        || snapshot.false_bad_apply != gate.false_bad_apply
        || snapshot.minimum_law_certificates != gate.min_law_certificates
        || snapshot.minimum_semantic_laws != gate.min_semantic_laws
        || snapshot.minimum_role_topologies != gate.min_role_topologies
        || snapshot.eligible_semantic_law_roots_sha256 != semantic_roots
        || snapshot.eligible_role_topology_roots_sha256 != topology_roots
        || snapshot.k1_open != gate.open
    {
        return Err("k1_candidate_freeze_registry_cas_failed".to_owned());
    }
    Ok(())
}

pub(super) fn send_authority_request<T: Serialize>(
    config: &CertificationAuthorityConfigV1,
    request: &T,
) -> Result<K1SchedulerProjectionV1, String> {
    #[cfg(not(unix))]
    {
        let _ = (config, request);
        return Err("k1_scheduler_authority_requires_unix".to_owned());
    }
    #[cfg(unix)]
    {
        let mut stream = UnixStream::connect(&config.authority_socket_path)
            .map_err(|error| format!("k1_scheduler_authority_connect:{error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|error| format!("k1_scheduler_authority_read_timeout:{error}"))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|error| format!("k1_scheduler_authority_write_timeout:{error}"))?;
        let bytes = serde_json::to_vec(request)
            .map_err(|error| format!("k1_scheduler_authority_encode:{error}"))?;
        if bytes.len() > K1_SCHEDULER_MAX_REQUEST_BYTES {
            return Err("k1_scheduler_authority_request_budget".to_owned());
        }
        stream
            .write_all(&bytes)
            .and_then(|_| stream.write_all(b"\n"))
            .map_err(|error| format!("k1_scheduler_authority_write:{error}"))?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(|error| format!("k1_scheduler_authority_shutdown:{error}"))?;
        let response: K1SchedulerAuthorityResponseV1 = serde_json::from_reader(&mut stream)
            .map_err(|error| format!("k1_scheduler_authority_decode:{error}"))?;
        if response.schema != K1_SCHEDULER_AUTHORITY_RESPONSE_SCHEMA_V1
            || !response.error.is_empty()
        {
            return Err(if response.error.is_empty() {
                "k1_scheduler_authority_response_invalid".to_owned()
            } else {
                response.error
            });
        }
        let projection = response
            .projection
            .ok_or_else(|| "k1_scheduler_authority_projection_missing".to_owned())?;
        projection.validate()?;
        Ok(projection)
    }
}
