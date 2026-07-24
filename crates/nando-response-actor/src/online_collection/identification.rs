//! Adaptive collection-law identification.
//!
//! This owner seals a completed bounded version space. It never grants runtime
//! authority; external admission rebuilds the same receipt from submitted
//! support before accepting a package.

use super::*;
use nando_operator_kernel::{
    OperatorGenerationComponentRootsV3, ProgramSemanticClassInputV1,
    seal_operator_generation_manifest_v3, seal_program_semantic_class_v1,
};
use nando_operator_learning::{
    CandidateFreezeReceiptV1, ExactProgramEvaluation, GenerationLearningOutcomeV3,
    OperatorIdentificationMachineV1, OperatorIdentificationStateV1, OperatorObservationInputV1,
    VersionSpaceConfig, seal_operator_observation_v1,
};

pub(super) struct CollectionIdentificationV1 {
    pub freeze: CandidateFreezeReceiptV1,
    pub program_sha256: String,
}

pub(super) fn adaptive_transfer_proof_root(
    future_manifest_sha256: &str,
    program_sha256: &str,
    program: &ResponseProgram,
    support_receipts: &[OnlineCollectionReceipt],
    future_receipts: &[OnlineCollectionReceipt],
    parity_receipts: &[DurableRuntimeParityReceipt],
) -> Result<String, String> {
    if future_receipts.is_empty() {
        return Err("collection_adaptive_future_missing".to_owned());
    }
    let future_refs = future_receipts
        .iter()
        .map(|receipt| receipt.evidence_graph_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let mut parity = parity_receipts
        .iter()
        .filter(|receipt| future_refs.contains(receipt.evidence_ref_sha256.as_str()))
        .collect::<Vec<_>>();
    parity.sort_by(|left, right| left.evidence_ref_sha256.cmp(&right.evidence_ref_sha256));
    if parity.len() != future_refs.len()
        || parity.iter().any(|receipt| {
            receipt.program_sha256 != program_sha256 || receipt.validate_sealed().is_err()
        })
    {
        return Err("collection_adaptive_transfer_parity_incomplete".to_owned());
    }
    let static_frame_transfer = if response_program_requires_static_frame_transfer(program) {
        let support_roots = dynamic_value_roots(program_sha256, support_receipts)?;
        let future_roots = dynamic_value_roots(program_sha256, future_receipts)?;
        if support_roots.is_empty()
            || future_roots.is_empty()
            || future_roots.difference(&support_roots).next().is_none()
        {
            return Err("collection_static_frame_transfer_unproven".to_owned());
        }
        Some(
            canonical_json_sha256(&(
                "nando.collection-static-frame-transfer.v1",
                program_sha256,
                support_roots,
                future_roots,
            ))
            .map_err(str::to_owned)?,
        )
    } else {
        None
    };
    canonical_json_sha256(&(
        "nando.collection-adaptive-transfer-proof.v1",
        future_manifest_sha256,
        program_sha256,
        static_frame_transfer,
        parity
            .iter()
            .map(|receipt| {
                (
                    receipt.evidence_ref_sha256.as_str(),
                    receipt.receipt_sha256.as_str(),
                )
            })
            .collect::<Vec<_>>(),
    ))
    .map_err(str::to_owned)
}

fn dynamic_value_roots(
    program_sha256: &str,
    receipts: &[OnlineCollectionReceipt],
) -> Result<BTreeSet<String>, String> {
    receipts
        .iter()
        .map(|receipt| {
            receipt
                .matched_program_dynamic_value_root_sha256
                .get(program_sha256)
                .filter(|root| is_sha256(root))
                .cloned()
                .ok_or_else(|| "collection_static_frame_dynamic_root_missing".to_owned())
        })
        .collect()
}

pub(super) fn identify_collection_bucket(
    bucket: &OnlineCollectionBucket,
) -> Result<Option<CollectionIdentificationV1>, String> {
    identify_collection_version_space(
        &bucket.archetype_id,
        &bucket.bucket_id,
        &bucket.programs,
        &bucket.support,
    )
}

pub(super) fn identify_collection_candidate(
    candidate: &OnlineCollectionAdmissionCandidate,
) -> Result<Option<CollectionIdentificationV1>, String> {
    let programs = candidate
        .identification_programs
        .iter()
        .map(|program| {
            canonical_json_sha256(program)
                .map(|digest| (digest, program.clone()))
                .map_err(str::to_owned)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    identify_collection_version_space(
        &candidate.archetype_id,
        &candidate.bucket_id,
        &programs,
        &candidate.support_receipts,
    )
}

fn identify_collection_version_space(
    archetype_id: &str,
    bucket_id: &str,
    programs: &BTreeMap<String, ResponseProgram>,
    support: &[OnlineCollectionReceipt],
) -> Result<Option<CollectionIdentificationV1>, String> {
    if programs.is_empty()
        || support.is_empty()
        || support
            .iter()
            .any(|receipt| !receipt.verifier_pass || receipt.event_time_unix_nanos.is_none())
    {
        return Ok(None);
    }

    let verifier_roots = programs
        .values()
        .map(|program| {
            source_neutral_verifier_for_program(program)
                .map_err(str::to_owned)
                .and_then(|verifier| canonical_json_sha256(&verifier).map_err(str::to_owned))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let program_roots = programs.keys().cloned().collect::<Vec<_>>();
    let manifest = seal_operator_generation_manifest_v3(
        1,
        None,
        OperatorGenerationComponentRootsV3 {
            artifact_set_sha256: canonical_json_sha256(&(
                "nando.collection-identification-candidates.v1",
                archetype_id,
                &program_roots,
            ))
            .map_err(str::to_owned)?,
            dispatch_index_sha256: canonical_json_sha256(&(
                "nando.collection-identification-dispatch.v1",
                bucket_id,
            ))
            .map_err(str::to_owned)?,
            actor_program_sha256: canonical_json_sha256(&program_roots).map_err(str::to_owned)?,
            renderer_program_sha256: canonical_json_sha256(&(
                "nando.collection-identification-renderers.v1",
                programs
                    .values()
                    .map(response_program_kind_code)
                    .collect::<Vec<_>>(),
            ))
            .map_err(str::to_owned)?,
            verifier_contract_sha256: canonical_json_sha256(&verifier_roots)
                .map_err(str::to_owned)?,
            capability_contract_sha256: canonical_json_sha256(&(
                "nando.collection-identification-capability.v1",
                archetype_id,
            ))
            .map_err(str::to_owned)?,
            resource_budget_sha256: canonical_json_sha256(&(
                "nando.collection-identification-budget.v1",
                crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS,
                support.len(),
            ))
            .map_err(str::to_owned)?,
        },
    )
    .map_err(|error| format!("collection_identification_manifest:{error:?}").to_lowercase())?;

    let mut machine = OperatorIdentificationMachineV1::new(manifest, VersionSpaceConfig::default());
    let mut registered = BTreeMap::new();
    for (expected_digest, program) in programs {
        let verifier = source_neutral_verifier_for_program(program).map_err(str::to_owned)?;
        let descriptor = seal_program_semantic_class_v1(ProgramSemanticClassInputV1 {
            effect_law_id_sha256: canonical_json_sha256(&(
                "nando.collection-effect-law.v1",
                archetype_id,
            ))
            .map_err(str::to_owned)?,
            role_schema_root_sha256: canonical_json_sha256(&(
                "nando.collection-role-schema.v1",
                response_program_required_routing_atom_ids(program),
            ))
            .map_err(str::to_owned)?,
            protocol_mode_set_root_sha256: canonical_json_sha256(&(
                "nando.collection-protocol-mode.v1",
                response_program_kind_code(program),
            ))
            .map_err(str::to_owned)?,
            // Distinct executable programs remain competing classes until
            // evidence eliminates them; semantic labels cannot collapse a tie.
            executable_behavior_root_sha256: expected_digest.clone(),
            verifier_contract_root_sha256: canonical_json_sha256(&verifier)
                .map_err(str::to_owned)?,
        })
        .map_err(|error| {
            format!("collection_identification_descriptor:{error:?}").to_lowercase()
        })?;
        let version_root = machine
            .register_candidate(program.clone(), descriptor)
            .map_err(|error| {
                format!("collection_identification_register:{error:?}").to_lowercase()
            })?;
        registered.insert(version_root, expected_digest.clone());
    }
    machine.complete_candidate_generation();

    let mut ordered_support = support.iter().collect::<Vec<_>>();
    ordered_support.sort_by(|left, right| {
        left.event_time_unix_nanos
            .cmp(&right.event_time_unix_nanos)
            .then_with(|| left.evidence_graph_sha256.cmp(&right.evidence_graph_sha256))
    });
    for (index, receipt) in ordered_support.into_iter().enumerate() {
        let evaluations = registered
            .iter()
            .map(|(version_root, canonical_digest)| ExactProgramEvaluation {
                program_digest_sha256: version_root.clone(),
                accepted: receipt.verifier_pass
                    && receipt.matched_program_sha256.contains(canonical_digest),
                reason: if receipt.matched_program_sha256.contains(canonical_digest) {
                    String::new()
                } else {
                    "exact_teacher_mismatch".to_owned()
                },
            })
            .collect();
        let observation = seal_operator_observation_v1(OperatorObservationInputV1 {
            capture_sequence: u64::try_from(index.saturating_add(1)).unwrap_or(u64::MAX),
            lineage_root_sha256: receipt.session_id_sha256.clone(),
            event_root_sha256: observation_root("event", receipt),
            request_root_sha256: observation_root("request", receipt),
            pre_action_relation_root_sha256: canonical_json_sha256(&(
                "nando.collection-pre-action.v1",
                &receipt.layout_sha256,
                &receipt.request_atom_ids,
            ))
            .map_err(str::to_owned)?,
            observed_action_root_sha256: canonical_json_sha256(&(
                "nando.collection-observed-action.v1",
                &receipt.matched_program_sha256,
            ))
            .map_err(str::to_owned)?,
            observed_delta_root_sha256: observation_root("delta", receipt),
            verifier_receipt_root_sha256: observation_root("verifier", receipt),
            outcome: GenerationLearningOutcomeV3::VerifiedPass,
            evaluations,
        })
        .map_err(|error| {
            format!("collection_identification_observation:{error:?}").to_lowercase()
        })?;
        machine.apply_support(observation).map_err(|error| {
            format!("collection_identification_support:{error:?}").to_lowercase()
        })?;
    }

    let identified = match machine
        .state()
        .map_err(|error| format!("collection_identification_state:{error:?}").to_lowercase())?
    {
        OperatorIdentificationStateV1::Identified { class } => class,
        OperatorIdentificationStateV1::Ambiguous { .. }
        | OperatorIdentificationStateV1::Collecting { .. } => return Ok(None),
        OperatorIdentificationStateV1::Empty { .. }
        | OperatorIdentificationStateV1::Exhausted { .. }
        | OperatorIdentificationStateV1::Contradicted { .. }
        | OperatorIdentificationStateV1::Frozen { .. } => return Ok(None),
    };
    let canonical_program_sha256 = registered
        .get(identified.canonical_program_root_sha256())
        .ok_or_else(|| "collection_identification_canonical_mapping_missing".to_owned())?;
    let canonical_program = programs
        .get(canonical_program_sha256)
        .ok_or_else(|| "collection_identification_canonical_program_missing".to_owned())?;
    let phase_centers = identification_phase_centers(programs, canonical_program, support);
    let scope_root = canonical_json_sha256(&(
        "nando.collection-applicability-scope.v1",
        bucket_id,
        identified.semantic_class().class_id().as_str(),
        identified.canonical_program_root_sha256(),
        &phase_centers,
    ))
    .map_err(str::to_owned)?;
    let freeze = machine
        .freeze_candidate(
            u64::try_from(support.len().saturating_add(1)).unwrap_or(u64::MAX),
            scope_root,
        )
        .map_err(|error| format!("collection_identification_freeze:{error:?}").to_lowercase())?;
    Ok(Some(CollectionIdentificationV1 {
        freeze: freeze.clone(),
        program_sha256: canonical_program_sha256.clone(),
    }))
}

fn identification_phase_centers(
    programs: &BTreeMap<String, ResponseProgram>,
    canonical_program: &ResponseProgram,
    support: &[OnlineCollectionReceipt],
) -> Vec<u64> {
    let all_program_atoms = programs
        .values()
        .flat_map(response_program_required_routing_atom_ids)
        .collect::<BTreeSet<_>>();
    let mut support_rows = support.iter();
    let mut common_pre_action = support_rows
        .next()
        .map(|receipt| {
            receipt
                .request_atom_ids
                .iter()
                .copied()
                .filter(|atom| !all_program_atoms.contains(atom))
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    for receipt in support_rows {
        let row = receipt
            .request_atom_ids
            .iter()
            .copied()
            .filter(|atom| !all_program_atoms.contains(atom))
            .collect::<BTreeSet<_>>();
        common_pre_action.retain(|atom| row.contains(atom));
    }
    let mut phase_centers = response_program_required_routing_atom_ids(canonical_program);
    phase_centers.extend(common_pre_action);
    phase_centers.sort_unstable();
    phase_centers.dedup();
    phase_centers
}

fn observation_root(domain: &str, receipt: &OnlineCollectionReceipt) -> String {
    canonical_json_sha256(&(
        "nando.collection-identification-observation-root.v1",
        domain,
        &receipt.evidence_graph_sha256,
        &receipt.client_intent_id_sha256,
    ))
    .expect("SHA-256 material is serializable")
}
