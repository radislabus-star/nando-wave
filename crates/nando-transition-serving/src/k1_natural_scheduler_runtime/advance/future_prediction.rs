use super::*;

pub(in crate::k1_natural_scheduler_runtime) enum FutureEvidenceAdvance {
    Pending {
        projection: K1SchedulerProjectionV1,
        state: K1NaturalSchedulerRuntimeStateV1,
        blocker: &'static str,
    },
    Ready(K1SchedulerProjectionV1),
}

pub(in crate::k1_natural_scheduler_runtime) fn durable_future_prediction_contract(
    identification: &K1IdentificationFreezeV1,
) -> bool {
    identification.prediction_schema == K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1
}

#[allow(clippy::too_many_arguments)]
pub(in crate::k1_natural_scheduler_runtime) fn advance_future_evidence(
    certification: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    projection: K1SchedulerProjectionV1,
    candidate: &K1NaturalCandidateFreezeV1,
    identification: &K1IdentificationFreezeV1,
    base_identification: &MultiSourceT1IdentificationV3,
    topologies: &[PreActionTopologyAuditRowV1],
    bindings: &[EvidenceBinding],
    frames: &[RelationFrame],
    terminal_receipts: &[TransportTerminalReceiptV1],
    candidate_artifacts: &[NaturalT1ProgramArtifactV1],
) -> Result<FutureEvidenceAdvance, String> {
    if lane != K1SchedulerLaneV1::Epistemic {
        return Ok(FutureEvidenceAdvance::Ready(projection));
    }
    if projection.future_prediction_contract.is_none() {
        let canonical_program = base_identification
            .canonical_program
            .clone()
            .ok_or_else(|| "k1_future_contract_program_missing".to_owned())?;
        let semantic_class_root_sha256 = current_classes(&projection)
            .into_iter()
            .next()
            .ok_or_else(|| "k1_future_contract_class_missing".to_owned())?;
        let protocol_mode_root_sha256 = base_identification
            .selected_protocol_mode_root_sha256
            .clone()
            .ok_or_else(|| "k1_future_contract_protocol_missing".to_owned())?;
        let projection = append_future_contract(
            certification,
            K1FutureContractAuthorityRequestV1 {
                schema: K1_FUTURE_CONTRACT_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
                lane,
                candidate_freeze_root_sha256: candidate.freeze_root_sha256.clone(),
                identification_freeze_root_sha256: identification.freeze_root_sha256.clone(),
                semantic_class_root_sha256,
                protocol_mode_root_sha256,
                canonical_program,
            },
        )?;
        return Ok(FutureEvidenceAdvance::Pending {
            projection,
            state: K1NaturalSchedulerRuntimeStateV1::FuturePredictionContractSealed,
            blocker: "future_prediction_contract_sealed",
        });
    }

    if let Some((projection, state, blocker)) = settle_precommitted_future_evidence(
        certification,
        lane,
        &projection,
        base_identification,
        topologies,
        bindings,
        frames,
        terminal_receipts,
        candidate_artifacts,
    )? {
        return Ok(FutureEvidenceAdvance::Pending {
            projection,
            state,
            blocker,
        });
    }

    let unresolved_prediction = projection.future_predictions.iter().any(|prediction| {
        !projection
            .future_outcomes
            .iter()
            .any(|outcome| outcome.prediction_root_sha256 == prediction.prediction_root_sha256)
            && !projection
                .future_prediction_censors
                .iter()
                .any(|receipt| receipt.prediction_root_sha256 == prediction.prediction_root_sha256)
    });
    if unresolved_prediction {
        return Ok(FutureEvidenceAdvance::Pending {
            projection,
            state: K1NaturalSchedulerRuntimeStateV1::AwaitingIndependentFuture,
            blocker: "durable_future_prediction_pending_outcome",
        });
    }

    let contract = projection
        .future_prediction_contract
        .as_ref()
        .ok_or_else(|| "k1_future_contract_missing_after_append".to_owned())?;
    if let Some(topology) = next_pre_action_topology(
        topologies,
        bindings,
        frames,
        &projection.future_predictions,
        candidate,
        &contract.canonical_program,
    )? {
        let projection = append_future_prediction(
            certification,
            K1FuturePredictionAuthorityRequestV1 {
                schema: K1_FUTURE_PREDICTION_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
                lane,
                contract_root_sha256: contract.contract_root_sha256.clone(),
                topology_commitment_root_sha256: topology.commit.commitment_root_sha256.clone(),
                provider_capture_request_root_sha256: topology
                    .commit
                    .provider_capture_request_root_sha256
                    .clone(),
            },
        )?;
        return Ok(FutureEvidenceAdvance::Pending {
            projection,
            state: K1NaturalSchedulerRuntimeStateV1::AwaitingIndependentFuture,
            blocker: "durable_future_prediction_pending_outcome",
        });
    }

    if projection
        .future_outcomes
        .iter()
        .any(|outcome| outcome.independent_verifier_pass)
    {
        Ok(FutureEvidenceAdvance::Ready(projection))
    } else {
        Ok(FutureEvidenceAdvance::Pending {
            projection,
            state: K1NaturalSchedulerRuntimeStateV1::AwaitingIndependentFuture,
            blocker: "independent_post_identification_future_pending",
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::k1_natural_scheduler_runtime) fn settle_precommitted_future_evidence(
    certification: &CertificationAuthorityConfigV1,
    lane: K1SchedulerLaneV1,
    projection: &K1SchedulerProjectionV1,
    base_identification: &MultiSourceT1IdentificationV3,
    topologies: &[PreActionTopologyAuditRowV1],
    bindings: &[EvidenceBinding],
    frames: &[RelationFrame],
    terminal_receipts: &[TransportTerminalReceiptV1],
    candidate_artifacts: &[NaturalT1ProgramArtifactV1],
) -> Result<
    Option<(
        K1SchedulerProjectionV1,
        K1NaturalSchedulerRuntimeStateV1,
        &'static str,
    )>,
    String,
> {
    if lane != K1SchedulerLaneV1::Epistemic {
        return Ok(None);
    }
    if let Some((prediction, topology, binding, frame, program_evidence)) = next_settleable_outcome(
        projection,
        topologies,
        bindings,
        frames,
        candidate_artifacts,
        base_identification.canonical_program.as_ref(),
    ) {
        let projection = append_future_outcome(
            certification,
            K1FutureOutcomeAuthorityRequestV1 {
                schema: K1_FUTURE_OUTCOME_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
                lane,
                prediction_root_sha256: prediction.prediction_root_sha256.clone(),
                topology: topology.clone(),
                frame: frame.clone(),
                program_evidence: program_evidence.cloned(),
            },
        )?;
        debug_assert_eq!(
            binding.topology_commitment_root_sha256,
            prediction.topology_commitment_root_sha256
        );
        return Ok(Some((
            projection,
            K1NaturalSchedulerRuntimeStateV1::FutureOutcomeSettled,
            "future_outcome_settled",
        )));
    }

    if let Some((prediction, fence)) =
        next_missing_completed_frame_censor(projection, topologies, frames, terminal_receipts)
    {
        let projection = append_future_prediction_censor(
            certification,
            K1FuturePredictionCensorAuthorityRequestV1 {
                schema: K1_FUTURE_PREDICTION_CENSOR_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
                lane,
                prediction_root_sha256: prediction.prediction_root_sha256.clone(),
                fence_topology_commitment_root_sha256: fence.commit.commitment_root_sha256.clone(),
                fence_provider_capture_request_root_sha256: fence
                    .commit
                    .provider_capture_request_root_sha256
                    .clone(),
            },
        )?;
        return Ok(Some((
            projection,
            K1NaturalSchedulerRuntimeStateV1::FuturePredictionCensored,
            K1_MISSING_COMPLETED_FRAME_BLOCKER_V1,
        )));
    }

    Ok(None)
}

fn next_settleable_outcome<'a>(
    projection: &'a K1SchedulerProjectionV1,
    topologies: &'a [PreActionTopologyAuditRowV1],
    bindings: &'a [EvidenceBinding],
    frames: &'a [RelationFrame],
    candidate_artifacts: &'a [NaturalT1ProgramArtifactV1],
    canonical_program: Option<&nando_operator_kernel::ResponseProgram>,
) -> Option<(
    &'a nando_operator_learning::multi_source::K1FuturePredictionReceiptV1,
    &'a PreActionTopologyAuditRowV1,
    &'a EvidenceBinding,
    &'a RelationFrame,
    Option<&'a NaturalT1ProgramArtifactV1>,
)> {
    let _ = (candidate_artifacts, canonical_program);
    projection.future_predictions.iter().find_map(|prediction| {
        if projection
            .future_outcomes
            .iter()
            .any(|outcome| outcome.prediction_root_sha256 == prediction.prediction_root_sha256)
            || projection
                .future_prediction_censors
                .iter()
                .any(|receipt| receipt.prediction_root_sha256 == prediction.prediction_root_sha256)
        {
            return None;
        }
        let topology = topologies.iter().find(|topology| {
            topology.commit.commitment_root_sha256 == prediction.topology_commitment_root_sha256
        })?;
        let binding = bindings.iter().find(|binding| {
            binding.topology_commitment_root_sha256 == prediction.topology_commitment_root_sha256
        })?;
        let frame = frames.iter().find(|frame| {
            canonical_json_sha256(*frame)
                .is_ok_and(|root| root == binding.completed_frame_root_sha256)
        })?;
        Some((prediction, topology, binding, frame, None))
    })
}

fn next_pre_action_topology<'a>(
    topologies: &'a [PreActionTopologyAuditRowV1],
    bindings: &[EvidenceBinding],
    frames: &[RelationFrame],
    predictions: &[nando_operator_learning::multi_source::K1FuturePredictionReceiptV1],
    candidate: &K1NaturalCandidateFreezeV1,
    program: &nando_operator_kernel::ResponseProgram,
) -> Result<Option<&'a PreActionTopologyAuditRowV1>, String> {
    if matches!(
        program.operation,
        nando_operator_kernel::ResponseOperation::ComposeCollection { .. }
    ) {
        return Ok(None);
    }
    let joined_topologies = bindings
        .iter()
        .map(|binding| binding.topology_commitment_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let completed_intents = frames
        .iter()
        .map(|frame| frame.client_intent_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let predicted_topologies = predictions
        .iter()
        .map(|prediction| prediction.topology_commitment_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let mut eligible = topologies
        .iter()
        .filter(|topology| {
            topology.commit.capture_sequence >= candidate.future_min_sequence
                && !joined_topologies.contains(topology.commit.commitment_root_sha256.as_str())
                && !predicted_topologies.contains(topology.commit.commitment_root_sha256.as_str())
                && !completed_intents.contains(topology.structure.turn_intent_id_sha256.as_str())
        })
        .filter_map(|topology| {
            let shape =
                pre_action_applicability_shape_root_v1(&topology.structure.topology).ok()?;
            let role_graph =
                candidate_topology_root(candidate, &topology.structure.topology).ok()?;
            (shape == candidate.candidate_structural_root_sha256
                && role_graph == candidate.source_neutral_topology_root_sha256
                && pre_action_t1_binding_root(program, &topology.structure.topology).is_ok())
            .then_some(topology)
        })
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| {
        left.commit
            .capture_sequence
            .cmp(&right.commit.capture_sequence)
            .then_with(|| {
                left.commit
                    .commitment_root_sha256
                    .cmp(&right.commit.commitment_root_sha256)
            })
    });
    Ok(eligible.into_iter().next())
}

fn next_missing_completed_frame_censor<'a>(
    projection: &'a K1SchedulerProjectionV1,
    topologies: &'a [PreActionTopologyAuditRowV1],
    frames: &[RelationFrame],
    terminal_receipts: &[TransportTerminalReceiptV1],
) -> Option<(
    &'a nando_operator_learning::multi_source::K1FuturePredictionReceiptV1,
    &'a PreActionTopologyAuditRowV1,
)> {
    projection
        .future_predictions
        .iter()
        .filter(|prediction| {
            !projection
                .future_outcomes
                .iter()
                .any(|outcome| outcome.prediction_root_sha256 == prediction.prediction_root_sha256)
                && !projection.future_prediction_censors.iter().any(|receipt| {
                    receipt.prediction_root_sha256 == prediction.prediction_root_sha256
                })
        })
        .filter_map(|prediction| {
            let topology = topologies.iter().find(|topology| {
                topology.commit.commitment_root_sha256 == prediction.topology_commitment_root_sha256
            })?;
            if frames
                .iter()
                .any(|frame| frame.client_intent_id_sha256 == prediction.turn_intent_id_sha256)
            {
                return None;
            }
            let terminal = terminal_receipts.iter().find(|receipt| {
                receipt.request_event_id_sha256 == topology.structure.request_event_id_sha256
                    && receipt.completed_at_unix_nanos > prediction.predicted_at_unix_nanos
            })?;
            let lineage = topology.session_lineage_sha256.as_deref()?;
            let fence = topologies
                .iter()
                .filter(|fence| {
                    fence.session_lineage_sha256.as_deref() == Some(lineage)
                        && fence.commit.capture_sequence > prediction.capture_sequence
                        && fence.captured_at_unix_ms.is_some_and(|captured| {
                            captured.saturating_mul(1_000_000) > terminal.completed_at_unix_nanos
                        })
                        && fence.structure.request_event_id_sha256
                            != topology.structure.request_event_id_sha256
                })
                .min_by(|left, right| {
                    left.commit
                        .capture_sequence
                        .cmp(&right.commit.capture_sequence)
                        .then_with(|| {
                            left.commit
                                .commitment_root_sha256
                                .cmp(&right.commit.commitment_root_sha256)
                        })
                })?;
            Some((prediction, fence))
        })
        .min_by(
            |(left_prediction, left_fence), (right_prediction, right_fence)| {
                left_prediction
                    .capture_sequence
                    .cmp(&right_prediction.capture_sequence)
                    .then_with(|| {
                        left_fence
                            .commit
                            .capture_sequence
                            .cmp(&right_fence.commit.capture_sequence)
                    })
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nando_operator_kernel::{
        LEARNING_REQUEST_STRUCTURE_SCHEMA_V2, LearningRequestStructureV2,
        MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1,
        MultiSourceExtractionStatusV1, MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1,
        MultiSourceTemporalClassV1, MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1,
        PreActionTopologyCommitV1, sha256_bytes,
    };
    use nando_operator_learning::multi_source::K1FuturePredictionReceiptV1;

    fn topology(
        label: &str,
        lineage: &str,
        capture_sequence: u64,
        captured_at_unix_ms: u64,
    ) -> PreActionTopologyAuditRowV1 {
        let lineage_root = sha256_bytes(lineage.as_bytes());
        let provider_root = sha256_bytes(format!("provider-{label}").as_bytes());
        let structure = LearningRequestStructureV2 {
            schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
            turn_intent_id_sha256: sha256_bytes(format!("intent-{label}").as_bytes()),
            request_event_id_sha256: sha256_bytes(label.as_bytes()),
            provider_bound_turn_identity: true,
            session_lineage_roots_sha256: vec![lineage_root.clone()],
            request_phase_atom_ids: vec![1],
            pre_action_context_atom_ids: vec![2],
            capability_atom_ids: vec![3],
            estimated_input_tokens: 10,
            provider_payload_bytes: 100,
            provider_capture_request_root_sha256: provider_root,
            decidability_reason_code: "pre_action_pending".to_owned(),
            topology: PreActionMultiSourceTopologyV1 {
                extraction_status: MultiSourceExtractionStatusV1::Complete,
                grounded_output_count: 1,
                output_part_count: 1,
                roles: vec![MultiSourceRoleNodeV1 {
                    local_role_id: 1,
                    source_ordinal: 0,
                    value_ordinal: 0,
                    type_class: MultiSourceTypeClassV1::Array,
                    container_class: MultiSourceContainerClassV1::Sequence,
                    cardinality_class: MultiSourceCardinalityClassV1::Many,
                    temporal_class: MultiSourceTemporalClassV1::Latest,
                    depth_bucket: 1,
                    structural_flags: 0,
                }],
                role_witnesses: vec![MultiSourceRoleWitnessV1 {
                    local_role_id: 1,
                    value_sha256: sha256_bytes(format!("value-{label}").as_bytes()),
                    request_reference_ordinal: None,
                    request_reference_ordinal_candidates: Vec::new(),
                }],
                relations: Vec::new(),
            },
        };
        let commit = PreActionTopologyCommitV1::seal(
            &structure,
            MultiSourceEvidenceOriginV1::FreshLive,
            sha256_bytes(format!("bridge-{label}").as_bytes()),
            sha256_bytes(format!("capture-{label}").as_bytes()),
            capture_sequence,
        )
        .expect("topology commit");
        PreActionTopologyAuditRowV1 {
            bridge_epoch_sha256: sha256_bytes(b"bridge-epoch"),
            bridge_sequence: Some(capture_sequence),
            record_sha256: Some(sha256_bytes(format!("record-{label}").as_bytes())),
            capture_epoch_sha256: Some(sha256_bytes(b"capture-epoch")),
            capture_event_sha256: Some(sha256_bytes(format!("event-{label}").as_bytes())),
            capture_receipt_sha256: Some(sha256_bytes(format!("receipt-{label}").as_bytes())),
            captured_at_unix_ms: Some(captured_at_unix_ms),
            session_lineage_sha256: Some(lineage_root),
            physical_order_proven: true,
            structure,
            commit,
        }
    }

    fn prediction(topology: &PreActionTopologyAuditRowV1) -> K1FuturePredictionReceiptV1 {
        K1FuturePredictionReceiptV1::seal(
            sha256_bytes(b"contract"),
            sha256_bytes(b"candidate"),
            sha256_bytes(b"identification"),
            sha256_bytes(b"semantic-class"),
            topology.commit.commitment_root_sha256.clone(),
            topology.commit.provider_capture_request_root_sha256.clone(),
            topology.structure.turn_intent_id_sha256.clone(),
            sha256_bytes(b"binding"),
            &sha256_bytes(b"program"),
            topology.commit.capture_sequence,
            topology.captured_at_unix_ms.expect("capture time"),
            topology
                .captured_at_unix_ms
                .expect("capture time")
                .saturating_mul(1_000_000)
                .saturating_add(100),
        )
        .expect("prediction")
    }

    fn projection(prediction: K1FuturePredictionReceiptV1) -> K1SchedulerProjectionV1 {
        K1SchedulerProjectionV1 {
            schema: "test".to_owned(),
            projection_root_sha256: sha256_bytes(b"projection"),
            ledger_revision: 1,
            ledger_root_sha256: sha256_bytes(b"ledger"),
            latest_event_root_sha256: sha256_bytes(b"event"),
            completed_generations: 0,
            completed_candidate_roots_sha256: Vec::new(),
            next_generation_sequence: 1,
            active_candidate_freeze: None,
            identification_freeze: None,
            future_prediction_contract: None,
            future_predictions: vec![prediction],
            future_prediction_censors: Vec::new(),
            future_outcomes: Vec::new(),
            latest_probe_round: None,
            completed_probe_rounds: 0,
            latest_applied_outcome: None,
            consumed_outcome_roots_sha256: Vec::new(),
            applied_outcome_roots_sha256: Vec::new(),
            remaining_probe_budget: None,
            latest_terminal_verdict: None,
            pending_terminal_transfer: None,
            latest_transfer_settlement: None,
            authority_ready: false,
            phase_mutation_allowed: false,
        }
    }

    #[test]
    fn passive_partition_schema_is_not_a_durable_future_precommit() {
        assert_ne!(
            K1_PREDICTION_SCHEMA_V1,
            K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1
        );
    }

    #[test]
    fn terminal_and_later_same_lineage_fence_select_missing_frame_censor() {
        let predicted_topology = topology("predicted-request", "lineage", 10, 1_000);
        let prediction = prediction(&predicted_topology);
        let terminal = TransportTerminalReceiptV1::seal(
            predicted_topology.structure.request_event_id_sha256.clone(),
            prediction.predicted_at_unix_nanos + 1,
            prediction.predicted_at_unix_nanos + 2,
            418,
        )
        .expect("terminal");
        let fence = topology("fence-request", "lineage", 11, 1_001);
        let projection = projection(prediction.clone());
        let topologies = [predicted_topology.clone(), fence.clone()];

        let selected = next_missing_completed_frame_censor(
            &projection,
            &topologies,
            &[],
            std::slice::from_ref(&terminal),
        )
        .expect("censor candidate");
        assert_eq!(
            selected.0.prediction_root_sha256,
            prediction.prediction_root_sha256
        );
        assert_eq!(
            selected.1.commit.commitment_root_sha256,
            fence.commit.commitment_root_sha256
        );

        let wrong_lineage = topology("wrong-lineage", "other", 12, 1_002);
        assert!(
            next_missing_completed_frame_censor(
                &projection,
                &[predicted_topology.clone(), wrong_lineage],
                &[],
                std::slice::from_ref(&terminal),
            )
            .is_none()
        );

        let early_fence = topology("early-fence", "lineage", 12, 1_000);
        assert!(
            next_missing_completed_frame_censor(
                &projection,
                &[predicted_topology.clone(), early_fence],
                &[],
                std::slice::from_ref(&terminal),
            )
            .is_none()
        );

        let completed_frame = RelationFrame {
            schema: "test".to_owned(),
            frame_id_sha256: sha256_bytes(b"frame"),
            event_id_sha256: sha256_bytes(b"frame-event"),
            client_intent_id_sha256: prediction.turn_intent_id_sha256,
            session_id_sha256: sha256_bytes(b"session"),
            observed_at_unix_nanos: terminal.completed_at_unix_nanos,
            estimated_input_tokens: 1,
            extractor_version: "test".to_owned(),
            verifier_label: Some(true),
            atoms: Vec::new(),
            evidence_ref_sha256: sha256_bytes(b"evidence"),
        };
        assert!(
            next_missing_completed_frame_censor(
                &projection,
                &[predicted_topology, fence],
                &[completed_frame],
                &[terminal],
            )
            .is_none()
        );
    }
}
