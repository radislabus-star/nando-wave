//! Capture-bound collection candidate crossing into external admission.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use nando_operator_kernel::canonical_json_sha256;
use nando_operator_learning::OnlineCollectionReceipt;

use crate::crystallized_operator::{
    DurableProgramCrystallizationProof, VerifiedCrystallizedOperator, decode_sha256,
};
use crate::{
    DurableRuntimeParityReceipt, OnlineCollectionAdmissionCandidate, VerifiedOperatorRestartBundle,
    response_independent_verifier_program_digest, sha256_bytes,
};

pub const CRYSTALLIZED_COLLECTION_ADMISSION_CANDIDATE_SCHEMA_V1: &str =
    "nando.crystallized-collection-admission-candidate.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrystallizedCollectionAdmissionCandidateV1 {
    schema: String,
    candidate: OnlineCollectionAdmissionCandidate,
    source_candidate_sha256: String,
    candidate_sha256: String,
    support_capture_root_sha256: String,
    future_capture_root_sha256: String,
    operator_page_sha256: String,
    operator_registry_sha256: String,
    crystallization_seal_sha256: String,
}

impl CrystallizedCollectionAdmissionCandidateV1 {
    pub fn seal(
        candidate: &OnlineCollectionAdmissionCandidate,
    ) -> Result<Option<Self>, &'static str> {
        let Some((support_capture_root_sha256, future_capture_root_sha256)) =
            capture_partition_roots(candidate)?
        else {
            return Ok(None);
        };
        let source_candidate_sha256 = source_candidate_sha256(candidate)?;
        let winner_seal_sha256 = crystallization_winner_seal(
            &source_candidate_sha256,
            &support_capture_root_sha256,
            &future_capture_root_sha256,
        )?;
        let restart_bundle = compile_collection_operator(
            candidate,
            &source_candidate_sha256,
            &support_capture_root_sha256,
            &future_capture_root_sha256,
            &winner_seal_sha256,
        )?;
        let operator_page_sha256 = sha256_bytes(restart_bundle.page_bytes());
        let operator_registry_sha256 = sha256_bytes(restart_bundle.registry_cbor());
        let mut candidate = candidate.clone();
        candidate.package.crystallized_operator = Some(restart_bundle);
        candidate.package.validate()?;
        let candidate_sha256 = canonical_json_sha256(&candidate)?;
        let crystallization_seal_sha256 = crystallization_seal(
            &source_candidate_sha256,
            &candidate_sha256,
            &support_capture_root_sha256,
            &future_capture_root_sha256,
            &operator_page_sha256,
            &operator_registry_sha256,
        )?;
        Ok(Some(Self {
            schema: CRYSTALLIZED_COLLECTION_ADMISSION_CANDIDATE_SCHEMA_V1.to_owned(),
            candidate,
            source_candidate_sha256,
            candidate_sha256,
            support_capture_root_sha256,
            future_capture_root_sha256,
            operator_page_sha256,
            operator_registry_sha256,
            crystallization_seal_sha256,
        }))
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != CRYSTALLIZED_COLLECTION_ADMISSION_CANDIDATE_SCHEMA_V1 {
            return Err("crystallized_collection_candidate_schema_invalid");
        }
        let Some((support_root, future_root)) = capture_partition_roots(&self.candidate)? else {
            return Err("crystallized_collection_capture_provenance_missing");
        };
        let source_candidate_sha256 = source_candidate_sha256(&self.candidate)?;
        let winner_seal_sha256 =
            crystallization_winner_seal(&source_candidate_sha256, &support_root, &future_root)?;
        let expected_bundle = compile_collection_operator(
            &self.candidate,
            &source_candidate_sha256,
            &support_root,
            &future_root,
            &winner_seal_sha256,
        )?;
        let embedded_bundle = self
            .candidate
            .package
            .crystallized_operator
            .as_ref()
            .ok_or("crystallized_collection_operator_missing")?;
        let candidate_sha256 = canonical_json_sha256(&self.candidate)?;
        let operator_page_sha256 = sha256_bytes(embedded_bundle.page_bytes());
        let operator_registry_sha256 = sha256_bytes(embedded_bundle.registry_cbor());
        if embedded_bundle != &expected_bundle
            || self.source_candidate_sha256 != source_candidate_sha256
            || self.candidate_sha256 != candidate_sha256
            || self.support_capture_root_sha256 != support_root
            || self.future_capture_root_sha256 != future_root
            || self.operator_page_sha256 != operator_page_sha256
            || self.operator_registry_sha256 != operator_registry_sha256
            || self.crystallization_seal_sha256
                != crystallization_seal(
                    &source_candidate_sha256,
                    &candidate_sha256,
                    &support_root,
                    &future_root,
                    &operator_page_sha256,
                    &operator_registry_sha256,
                )?
        {
            return Err("crystallized_collection_candidate_seal_mismatch");
        }
        self.candidate.package.validate()?;
        Ok(())
    }

    #[must_use]
    pub fn candidate(&self) -> &OnlineCollectionAdmissionCandidate {
        &self.candidate
    }

    #[must_use]
    pub fn seal_sha256(&self) -> &str {
        &self.crystallization_seal_sha256
    }
}

fn capture_partition_roots(
    candidate: &OnlineCollectionAdmissionCandidate,
) -> Result<Option<(String, String)>, &'static str> {
    let Some(adaptive_proof) = &candidate.package.proof.adaptive_identification else {
        return Ok(None);
    };
    if adaptive_proof.validate().is_err()
        || candidate
            .candidate_freeze
            .as_ref()
            .is_none_or(|freeze| freeze.validate().is_err())
    {
        return Err("crystallized_collection_adaptive_proof_invalid");
    }
    if candidate.support_receipts.is_empty() || candidate.future_receipts.is_empty() {
        return Ok(None);
    }
    if candidate
        .support_receipts
        .iter()
        .chain(&candidate.future_receipts)
        .any(|receipt| receipt.capture_binding.is_none())
    {
        return Ok(None);
    }
    let support_root = capture_partition_root("support", &candidate.support_receipts)?;
    let future_root = capture_partition_root("future", &candidate.future_receipts)?;
    let support_frames = candidate
        .support_receipts
        .iter()
        .map(|receipt| receipt.evidence_graph_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let support_bindings = candidate
        .support_receipts
        .iter()
        .filter_map(|receipt| receipt.capture_binding.as_ref())
        .map(|binding| binding.record_sha256.as_str())
        .collect::<BTreeSet<_>>();
    if candidate.future_receipts.iter().any(|receipt| {
        support_frames.contains(receipt.evidence_graph_sha256.as_str())
            || receipt
                .capture_binding
                .as_ref()
                .is_some_and(|binding| support_bindings.contains(binding.record_sha256.as_str()))
    }) {
        return Err("crystallized_collection_capture_partition_overlap");
    }
    let support_max_binding = partition_sequence_bound(&candidate.support_receipts, true, false)?;
    let future_min_binding = partition_sequence_bound(&candidate.future_receipts, false, false)?;
    let support_max_source = partition_sequence_bound(&candidate.support_receipts, true, true)?;
    let future_min_source = partition_sequence_bound(&candidate.future_receipts, false, true)?;
    if support_max_binding >= future_min_binding || support_max_source >= future_min_source {
        return Err("crystallized_collection_capture_partition_reordered");
    }
    Ok(Some((support_root, future_root)))
}

fn capture_partition_root(
    partition: &str,
    receipts: &[OnlineCollectionReceipt],
) -> Result<String, &'static str> {
    let rows = receipts
        .iter()
        .map(|receipt| {
            let binding = receipt
                .capture_binding
                .as_ref()
                .ok_or("crystallized_collection_capture_provenance_missing")?;
            binding.verify_digest()?;
            if binding.frame_id_sha256 != receipt.evidence_graph_sha256 {
                return Err("crystallized_collection_capture_binding_mismatch");
            }
            Ok((
                receipt.evidence_graph_sha256.as_str(),
                binding.sequence,
                binding.records_root_sha256.as_str(),
                binding.source_record.sequence,
                binding.source_record.record_sha256.as_str(),
                binding.record_sha256.as_str(),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    canonical_json_sha256(&(
        CRYSTALLIZED_COLLECTION_ADMISSION_CANDIDATE_SCHEMA_V1,
        partition,
        rows,
    ))
}

fn partition_sequence_bound(
    receipts: &[OnlineCollectionReceipt],
    maximum: bool,
    source_record: bool,
) -> Result<u64, &'static str> {
    let mut sequences = receipts.iter().map(|receipt| {
        let binding = receipt
            .capture_binding
            .as_ref()
            .ok_or("crystallized_collection_capture_provenance_missing")?;
        Ok(if source_record {
            binding.source_record.sequence
        } else {
            binding.sequence
        })
    });
    let first = sequences
        .next()
        .ok_or("crystallized_collection_capture_partition_empty")??;
    sequences.try_fold(first, |bound, sequence| {
        sequence.map(|sequence| {
            if maximum {
                bound.max(sequence)
            } else {
                bound.min(sequence)
            }
        })
    })
}

fn crystallization_seal(
    source_candidate_sha256: &str,
    candidate_sha256: &str,
    support_capture_root_sha256: &str,
    future_capture_root_sha256: &str,
    operator_page_sha256: &str,
    operator_registry_sha256: &str,
) -> Result<String, &'static str> {
    canonical_json_sha256(&(
        CRYSTALLIZED_COLLECTION_ADMISSION_CANDIDATE_SCHEMA_V1,
        source_candidate_sha256,
        candidate_sha256,
        support_capture_root_sha256,
        future_capture_root_sha256,
        operator_page_sha256,
        operator_registry_sha256,
    ))
}

fn source_candidate_sha256(
    candidate: &OnlineCollectionAdmissionCandidate,
) -> Result<String, &'static str> {
    let mut source = candidate.clone();
    source.package.crystallized_operator = None;
    canonical_json_sha256(&("nando.collection-operator-source-candidate.v1", source))
}

fn crystallization_winner_seal(
    source_candidate_sha256: &str,
    support_capture_root_sha256: &str,
    future_capture_root_sha256: &str,
) -> Result<String, &'static str> {
    canonical_json_sha256(&(
        "nando.collection-operator-winner-seal.v1",
        source_candidate_sha256,
        support_capture_root_sha256,
        future_capture_root_sha256,
    ))
}

fn compile_collection_operator(
    candidate: &OnlineCollectionAdmissionCandidate,
    source_candidate_sha256: &str,
    support_capture_root_sha256: &str,
    future_capture_root_sha256: &str,
    winner_seal_sha256: &str,
) -> Result<VerifiedOperatorRestartBundle, &'static str> {
    validate_compilation_evidence(candidate)?;
    let verifier = candidate
        .package
        .verifier
        .clone()
        .ok_or("crystallized_collection_verifier_missing")?;
    let support_lineages = receipt_lineages(&candidate.support_receipts)?;
    let future_lineages = receipt_lineages(&candidate.future_receipts)?;
    if !support_lineages.is_disjoint(&future_lineages) {
        return Err("crystallized_collection_lineage_overlap");
    }
    let future_lineages = future_lineages.into_iter().collect::<Vec<_>>();
    let support_lineages = support_lineages.into_iter().collect::<Vec<_>>();
    let future_lineage_root_sha256 = canonical_json_sha256(&(
        "nando.collection-operator-future-lineages.v1",
        &future_lineages,
    ))?;
    let blueprint_sha256 = canonical_json_sha256(&(
        "nando.collection-operator-blueprint.v1",
        &candidate.package.program,
        &verifier,
        source_candidate_sha256,
    ))?;
    let candidate_set_sha256 = candidate
        .candidate_freeze
        .as_ref()
        .ok_or("crystallized_collection_freeze_missing")?
        .freeze_root_sha256();
    let parity = durable_parity_commitments(candidate, &verifier)?;
    let proof = DurableProgramCrystallizationProof {
        generation: candidate
            .candidate_freeze
            .as_ref()
            .expect("validated freeze")
            .support_watermark_next_sequence(),
        blueprint_sha256: decode_sha256(&blueprint_sha256).map_err(crystallization_error)?,
        candidate_set_sha256: decode_sha256(candidate_set_sha256).map_err(crystallization_error)?,
        support_root_sha256: decode_sha256(support_capture_root_sha256)
            .map_err(crystallization_error)?,
        future_evidence_root_sha256: decode_sha256(future_capture_root_sha256)
            .map_err(crystallization_error)?,
        future_lineage_root_sha256: decode_sha256(&future_lineage_root_sha256)
            .map_err(crystallization_error)?,
        winner_seal_sha256: decode_sha256(winner_seal_sha256).map_err(crystallization_error)?,
        support_lineages,
        future_lineages,
        binding_receipts: parity.binding,
        execution_receipts: parity.execution,
    };
    let operator = VerifiedCrystallizedOperator::crystallize_durable_program(
        candidate.package.program.clone(),
        verifier,
        proof,
    )
    .map_err(crystallization_error)?;
    let bundle = operator.restart_bundle().map_err(crystallization_error)?;
    let restored =
        VerifiedCrystallizedOperator::restore(bundle.page_bytes(), bundle.registry_cbor())
            .map_err(crystallization_error)?;
    if !operator.execution_equivalent(&restored)
        || operator.actor_sha256() != restored.actor_sha256()
        || operator.verifier_sha256() != restored.verifier_sha256()
    {
        return Err("crystallized_collection_restart_mismatch");
    }
    Ok(bundle)
}

fn validate_compilation_evidence(
    candidate: &OnlineCollectionAdmissionCandidate,
) -> Result<(), &'static str> {
    candidate.package.validate()?;
    let freeze = candidate
        .candidate_freeze
        .as_ref()
        .ok_or("crystallized_collection_freeze_missing")?;
    freeze
        .validate()
        .map_err(|_| "crystallized_collection_freeze_invalid")?;
    let adaptive = candidate
        .package
        .proof
        .adaptive_identification
        .as_ref()
        .ok_or("crystallized_collection_adaptive_proof_missing")?;
    adaptive.validate()?;
    if candidate.causal_report.verdict != "PASS"
        || candidate.causal_report.package_id != candidate.package.package_id
        || candidate.causal_report.wrong_accepts != 0
        || candidate.causal_report.full_phase_correct != candidate.future_receipts.len()
        || candidate.package.proof.support_rows != candidate.support_receipts.len()
        || candidate.package.proof.future_rows != candidate.future_receipts.len()
        || candidate.package.proof.wrong_accepts != 0
        || candidate.package.proof.runtime_parity_failures != 0
        || candidate.support_receipts.is_empty()
        || candidate.future_receipts.is_empty()
        || candidate
            .support_receipts
            .iter()
            .chain(&candidate.future_receipts)
            .any(|receipt| !receipt.verifier_pass)
    {
        return Err("crystallized_collection_proof_incomplete");
    }
    Ok(())
}

fn receipt_lineages(
    receipts: &[OnlineCollectionReceipt],
) -> Result<BTreeSet<[u8; 32]>, &'static str> {
    receipts
        .iter()
        .map(|receipt| {
            decode_sha256(&receipt.session_id_sha256)
                .map_err(|_| "crystallized_collection_session_lineage_invalid")
        })
        .collect()
}

struct DurableParityCommitments {
    binding: Vec<[u8; 32]>,
    execution: Vec<[u8; 32]>,
}

fn durable_parity_commitments(
    candidate: &OnlineCollectionAdmissionCandidate,
    verifier: &crate::VerifierProgram,
) -> Result<DurableParityCommitments, &'static str> {
    let program_sha256 = canonical_json_sha256(&candidate.package.program)?;
    let verifier_sha256 = response_independent_verifier_program_digest(verifier)?;
    let future_refs = candidate
        .future_receipts
        .iter()
        .map(|receipt| receipt.evidence_graph_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let mut receipts_by_evidence = BTreeMap::new();
    for receipt in &candidate.durable_runtime_parity_receipts {
        validate_durable_receipt(receipt, &program_sha256, &verifier_sha256)?;
        if !future_refs.contains(receipt.evidence_ref_sha256.as_str())
            || receipts_by_evidence
                .insert(receipt.evidence_ref_sha256.as_str(), receipt)
                .is_some()
        {
            return Err("crystallized_collection_parity_partition_invalid");
        }
    }
    if receipts_by_evidence.len() != future_refs.len() {
        return Err("crystallized_collection_parity_receipt_missing");
    }
    let mut binding_receipts = Vec::with_capacity(receipts_by_evidence.len());
    let mut execution_receipts = Vec::with_capacity(receipts_by_evidence.len());
    for evidence_ref in future_refs {
        let receipt = receipts_by_evidence
            .get(evidence_ref)
            .ok_or("crystallized_collection_parity_receipt_missing")?;
        let binding_sha256 = canonical_json_sha256(&(
            "nando.collection-operator-binding-receipt.v1",
            receipt.evidence_ref_sha256.as_str(),
            receipt.input_sha256.as_str(),
        ))?;
        binding_receipts.push(decode_sha256(&binding_sha256).map_err(crystallization_error)?);
        execution_receipts
            .push(decode_sha256(&receipt.receipt_sha256).map_err(crystallization_error)?);
    }
    Ok(DurableParityCommitments {
        binding: binding_receipts,
        execution: execution_receipts,
    })
}

fn validate_durable_receipt(
    receipt: &DurableRuntimeParityReceipt,
    program_sha256: &str,
    verifier_sha256: &str,
) -> Result<(), &'static str> {
    receipt.validate_sealed()?;
    if receipt.program_sha256 != program_sha256
        || receipt.verifier_sha256 != verifier_sha256
        || receipt.teacher_response_sha256 != receipt.actor_response_sha256
    {
        return Err("crystallized_collection_parity_receipt_mismatch");
    }
    Ok(())
}

fn crystallization_error(_error: impl std::fmt::Debug) -> &'static str {
    "crystallized_collection_operator_compile_failed"
}
