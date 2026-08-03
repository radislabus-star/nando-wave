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

    if let Some((prediction, topology, binding, frame, program_evidence)) = next_settleable_outcome(
        &projection,
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
            binding.joined.topology_commitment_root_sha256,
            prediction.topology_commitment_root_sha256
        );
        return Ok(FutureEvidenceAdvance::Pending {
            projection,
            state: K1NaturalSchedulerRuntimeStateV1::FutureOutcomeSettled,
            blocker: "future_outcome_settled",
        });
    }

    let unresolved_prediction = projection.future_predictions.iter().any(|prediction| {
        !projection
            .future_outcomes
            .iter()
            .any(|outcome| outcome.prediction_root_sha256 == prediction.prediction_root_sha256)
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
    if let Some((topology, pre_action_execution_receipt)) = next_pre_action_topology(
        certification,
        topologies,
        bindings,
        frames,
        candidate,
        &contract.canonical_program,
    )? {
        let projection = append_future_prediction(
            certification,
            K1FuturePredictionAuthorityRequestV1 {
                schema: K1_FUTURE_PREDICTION_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
                lane,
                contract_root_sha256: contract.contract_root_sha256.clone(),
                topology: topology.clone(),
                pre_action_execution_receipt,
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
        {
            return None;
        }
        let topology = topologies.iter().find(|topology| {
            topology.commit.commitment_root_sha256 == prediction.topology_commitment_root_sha256
        })?;
        let binding = bindings.iter().find(|binding| {
            binding.joined.topology_commitment_root_sha256
                == prediction.topology_commitment_root_sha256
        })?;
        let frame = frames.iter().find(|frame| {
            canonical_json_sha256(*frame)
                .is_ok_and(|root| root == binding.joined.completed_frame_root_sha256)
        })?;
        Some((prediction, topology, binding, frame, None))
    })
}

fn next_pre_action_topology<'a>(
    certification: &CertificationAuthorityConfigV1,
    topologies: &'a [PreActionTopologyAuditRowV1],
    bindings: &[EvidenceBinding],
    frames: &[RelationFrame],
    candidate: &K1NaturalCandidateFreezeV1,
    program: &nando_operator_kernel::ResponseProgram,
) -> Result<
    Option<(
        &'a PreActionTopologyAuditRowV1,
        Option<nando_operator_learning::multi_source::K1PreActionExecutionReceiptV1>,
    )>,
    String,
> {
    let joined_topologies = bindings
        .iter()
        .map(|binding| binding.joined.topology_commitment_root_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let completed_intents = frames
        .iter()
        .map(|frame| frame.client_intent_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let mut eligible = topologies
        .iter()
        .filter(|topology| {
            topology.commit.capture_sequence >= candidate.future_min_sequence
                && !joined_topologies.contains(topology.commit.commitment_root_sha256.as_str())
                && !completed_intents.contains(topology.structure.turn_intent_id_sha256.as_str())
        })
        .filter_map(|topology| {
            let shape =
                pre_action_applicability_shape_root_v1(&topology.structure.topology).ok()?;
            let role_graph = source_neutral_topology_root_v1(&topology.structure.topology).ok()?;
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
    let requires_typed_prediction = matches!(
        program.operation,
        nando_operator_kernel::ResponseOperation::ComposeCollection { .. }
    );
    let program_root = nando_operator_kernel::response_program_version_root_sha256(program)
        .map_err(str::to_owned)?;
    for topology in eligible {
        let receipt = crate::k1_pre_action_prediction::restore_for_request(
            certification,
            &topology.commit.provider_capture_request_root_sha256,
            &program_root,
        )?;
        if !requires_typed_prediction || receipt.is_some() {
            return Ok(Some((topology, receipt)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passive_partition_schema_is_not_a_durable_future_precommit() {
        assert_ne!(
            K1_PREDICTION_SCHEMA_V1,
            K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1
        );
    }
}
