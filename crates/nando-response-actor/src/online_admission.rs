use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{
    PhaseCenterCell, PhaseCenterFlatRuntime, phase_margin_to_micro, phase_vector_from_atom_ids,
};
use serde::Serialize;

use crate::teacher_join::action_schema_enriched_frame;
use crate::{
    COMPOSITE_ADMISSION_SCHEMA_V2, CollectionSynthesisExample, CompositeResponseAdmissionV2,
    DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1, DurableRuntimeParityReceipt, LearnedWaveRoute,
    LearnedWaveSubcenter, OnlineCollectionAdmissionCandidate, OnlineResponseAdmissionCandidate,
    RESPONSE_AUTHORITY_SCHEMA_V2, RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2,
    RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2, RESPONSE_REGISTRY_SCHEMA_V6,
    RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1, RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1,
    RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1, ResponseAuthorityV2, ResponseExecutionStatus,
    ResponsePackage, ResponsePackageAuthorityBindingV2, ResponsePackageState, ResponseProgram,
    ResponseRegistry, VerifiedCrystallizedOperator, canonical_json_sha256,
    compile_source_neutral_quarantine_packages, evaluate_grounded_wave_causality, execute_response,
    frame_matches_program_action_contract, online_collection_future_manifest_digest,
    online_collection_support_manifest_digest, relation_frame_routes_to_package,
    relation_frame_structural_family_id, response_actor_program_digest,
    response_execution_payload_digest, response_independent_verifier_program_digest,
    response_package_digest, response_program_authority_matches_example,
    response_program_required_routing_atom_ids, response_proof_receipts_digest,
    response_registry_digest, sha256_bytes, source_neutral_verifier_for_program,
    valid_nonzero_sha256, verify_response_independently,
};

use crate::{LiveScalarAdmissionCandidate, LiveScalarShadowState};

#[derive(Clone, Debug)]
pub struct OnlineAdmissionSnapshot {
    pub registry: ResponseRegistry,
    pub admission: CompositeResponseAdmissionV2,
}

pub fn build_crystallized_admission_snapshot(
    candidates: &[LiveScalarAdmissionCandidate],
    project_id: &str,
    revision: u64,
    now_unix: u64,
    max_age_seconds: u64,
    gate_build_sha256: &str,
    runtime_build_sha256: &str,
) -> Result<Option<OnlineAdmissionSnapshot>, &'static str> {
    let mut packages = Vec::new();
    let mut receipts = BTreeMap::new();
    for submitted in candidates {
        if submitted.support.len() != 32 || submitted.future.len() != 32 {
            return Err("crystallized_admission_evidence_window_invalid");
        }
        // A deserialized candidate is only an evidence envelope. Rebuild the
        // winner, causal controls, executable seals and package from the 64
        // bounded rows so caller-provided proof counters never gain authority.
        let mut replay = LiveScalarShadowState::default();
        for row in submitted.support.iter().chain(&submitted.future) {
            replay.observe(row);
        }
        let rebuilt = replay.admission_candidates();
        let [candidate] = rebuilt.as_slice() else {
            return Err("crystallized_admission_resynthesis_failed");
        };
        if submitted.package != candidate.package
            || submitted.support_root_sha256 != candidate.support_root_sha256
            || submitted.future_evidence_root_sha256 != candidate.future_evidence_root_sha256
            || submitted.future_lineage_root_sha256 != candidate.future_lineage_root_sha256
            || submitted.winner_seal_sha256 != candidate.winner_seal_sha256
            || submitted.executable_parity_seal_sha256 != candidate.executable_parity_seal_sha256
        {
            return Err("crystallized_admission_resynthesis_mismatch");
        }
        let mut package = candidate.package.clone();
        if candidate.support.len() != package.proof.support_rows
            || candidate.future.len() != package.proof.future_rows
            || package.proof.support_rows < 32
            || package.proof.future_rows < 32
            || package.proof.distinct_sessions < 3
            || package.proof.distinct_surfaces < 2
            || package.proof.wrong_accepts != 0
            || package.proof.runtime_parity_failures != 0
            || package.proof.exact_cache_overlap != 0
            || !package.proof.wave_causal_pass
        {
            return Err("crystallized_admission_proof_gate_failed");
        }
        let Some(bundle) = &package.crystallized_operator else {
            return Err("crystallized_admission_bundle_missing");
        };
        let operator =
            VerifiedCrystallizedOperator::restore(bundle.page_bytes(), bundle.registry_cbor())
                .map_err(|_| "crystallized_admission_restore_failed")?;
        let support_sessions = candidate
            .support
            .iter()
            .map(|row| row.before.session_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        if candidate
            .future
            .iter()
            .any(|row| support_sessions.contains(row.before.session_id_sha256.as_str()))
        {
            return Err("crystallized_admission_support_future_session_overlap");
        }
        let mut future_lineages = BTreeSet::new();
        let mut replay_failed = false;
        for (is_future, row) in candidate
            .support
            .iter()
            .map(|row| (false, row))
            .chain(candidate.future.iter().map(|row| (true, row)))
        {
            let sample = match crate::extract_live_scalar_circuit_sample(row) {
                Ok(sample) => sample,
                Err(_) => {
                    replay_failed = true;
                    break;
                }
            };
            if is_future {
                future_lineages.insert(*sample.bundle.lineage_sha256());
            }
            let response = operator
                .bind_pre_action(&sample.request_text, &sample.provider_payload)
                .and_then(|bound| bound.execute_verified());
            if response.as_deref() != Ok(sample.expected_response.as_str()) {
                replay_failed = true;
                break;
            }
        }
        if replay_failed
            || future_lineages
                != operator
                    .verified_future_lineages()
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>()
        {
            return Err("crystallized_admission_replay_failed");
        }
        let commitments = [
            candidate.support_root_sha256.as_str(),
            candidate.future_evidence_root_sha256.as_str(),
            candidate.future_lineage_root_sha256.as_str(),
            candidate.winner_seal_sha256.as_str(),
            candidate.executable_parity_seal_sha256.as_str(),
        ];
        if commitments
            .iter()
            .any(|digest| !valid_nonzero_sha256(digest))
            || candidate.support_root_sha256 != commitment_hex(operator.support_root_sha256())
            || candidate.future_evidence_root_sha256
                != commitment_hex(operator.future_evidence_root_sha256())
            || candidate.future_lineage_root_sha256
                != commitment_hex(operator.future_lineage_root_sha256())
            || candidate.winner_seal_sha256 != commitment_hex(operator.winner_seal_sha256())
            || candidate.executable_parity_seal_sha256
                != commitment_hex(operator.parity_seal().seal_sha256())
            || operator.parity_seal().future_lineage_count() < 32
            || operator.parity_seal().wrong_accepts() != 0
        {
            return Err("crystallized_admission_commitment_mismatch");
        }
        package.validate()?;
        package.state = ResponsePackageState::Active;
        let support_manifest_sha256 = canonical_json_sha256(&(
            RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1,
            candidate
                .support
                .iter()
                .map(|row| row.before.frame_id_sha256.as_str())
                .collect::<Vec<_>>(),
        ))?;
        let future_manifest_sha256 = canonical_json_sha256(&(
            RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2,
            candidate
                .future
                .iter()
                .map(|row| row.before.evidence_ref_sha256.as_str())
                .collect::<Vec<_>>(),
        ))?;
        receipts.insert(
            package.package_id.clone(),
            (
                support_manifest_sha256,
                candidate.winner_seal_sha256.clone(),
                candidate.executable_parity_seal_sha256.clone(),
                future_manifest_sha256,
                candidate.future_lineage_root_sha256.clone(),
            ),
        );
        packages.push(package);
    }
    if packages.is_empty() {
        return Ok(None);
    }
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    if packages
        .windows(2)
        .any(|pair| pair[0].package_id == pair[1].package_id)
    {
        return Err("crystallized_admission_duplicate_package_id");
    }
    let registry = ResponseRegistry {
        schema: RESPONSE_REGISTRY_SCHEMA_V6.to_owned(),
        revision,
        packages,
    };
    registry.validate()?;
    let registry_sha256 = response_registry_digest(&registry)?;
    let mut bindings = Vec::new();
    for package in &registry.packages {
        let (support, causal, parity, future, lineage) = receipts
            .remove(&package.package_id)
            .ok_or("crystallized_admission_receipts_missing")?;
        let verifier = package
            .verifier
            .as_ref()
            .ok_or("crystallized_admission_verifier_missing")?;
        let mut binding = ResponsePackageAuthorityBindingV2 {
            package_id: package.package_id.clone(),
            registry_revision: revision,
            package_sha256: response_package_digest(package)?,
            execution_payload_sha256: response_execution_payload_digest(package)?,
            actor_program_sha256: response_actor_program_digest(&package.program)?,
            independent_verifier_program_sha256: response_independent_verifier_program_digest(
                verifier,
            )?,
            verifier_schema: package.proof.verifier_schema.clone(),
            support_manifest_schema: RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1.to_owned(),
            support_manifest_sha256: support,
            exact_causal_proof_schema: RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2.to_owned(),
            exact_causal_proof_sha256: causal,
            runtime_parity_receipt_set_schema: RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1
                .to_owned(),
            runtime_parity_receipt_set_sha256: parity,
            future_verifier_receipt_set_schema: RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2
                .to_owned(),
            future_verifier_receipt_set_sha256: future,
            semantic_alias_proof_schema: RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1.to_owned(),
            semantic_alias_proof_sha256: lineage,
            proof_receipts_sha256: String::new(),
        };
        binding.proof_receipts_sha256 = response_proof_receipts_digest(&binding)?;
        bindings.push(binding);
    }
    let admission = CompositeResponseAdmissionV2 {
        schema: COMPOSITE_ADMISSION_SCHEMA_V2.to_owned(),
        project_id: project_id.to_owned(),
        generated_at_unix: now_unix,
        expires_at_unix: now_unix.saturating_add(max_age_seconds),
        verdict: "PASS".to_owned(),
        eligible_for_local_accept: true,
        response_authority: ResponseAuthorityV2 {
            schema: RESPONSE_AUTHORITY_SCHEMA_V2.to_owned(),
            registry_schema: registry.schema.clone(),
            registry_revision: revision,
            registry_sha256,
            gate_build_sha256: gate_build_sha256.to_owned(),
            runtime_build_sha256: runtime_build_sha256.to_owned(),
            packages: bindings,
        },
    };
    Ok(Some(OnlineAdmissionSnapshot {
        registry,
        admission,
    }))
}

fn commitment_hex(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn merge_online_admission_snapshots(
    snapshots: Vec<OnlineAdmissionSnapshot>,
) -> Result<Option<OnlineAdmissionSnapshot>, &'static str> {
    let mut snapshots = snapshots.into_iter();
    let Some(first) = snapshots.next() else {
        return Ok(None);
    };
    let mut registry = first.registry;
    let mut admission = first.admission;
    for snapshot in snapshots {
        if snapshot.registry.schema != registry.schema
            || snapshot.registry.revision != registry.revision
            || snapshot.admission.schema != admission.schema
            || snapshot.admission.project_id != admission.project_id
            || snapshot.admission.generated_at_unix != admission.generated_at_unix
            || snapshot.admission.expires_at_unix != admission.expires_at_unix
            || snapshot.admission.verdict != admission.verdict
            || snapshot.admission.eligible_for_local_accept != admission.eligible_for_local_accept
            || snapshot.admission.response_authority.gate_build_sha256
                != admission.response_authority.gate_build_sha256
            || snapshot.admission.response_authority.runtime_build_sha256
                != admission.response_authority.runtime_build_sha256
        {
            return Err("online_admission_snapshot_contract_mismatch");
        }
        registry.packages.extend(snapshot.registry.packages);
        admission
            .response_authority
            .packages
            .extend(snapshot.admission.response_authority.packages);
    }
    registry
        .packages
        .sort_by(|left, right| left.package_id.cmp(&right.package_id));
    for duplicate in registry.packages.windows(2) {
        if duplicate[0].package_id == duplicate[1].package_id {
            return Err("online_admission_duplicate_package_id");
        }
    }
    admission
        .response_authority
        .packages
        .sort_by(|left, right| left.package_id.cmp(&right.package_id));
    for duplicate in admission.response_authority.packages.windows(2) {
        if duplicate[0].package_id == duplicate[1].package_id {
            return Err("online_admission_duplicate_authority_binding");
        }
    }
    let revision = authority_content_revision(&registry, &admission.response_authority.packages)?;
    registry.revision = revision;
    for binding in &mut admission.response_authority.packages {
        binding.registry_revision = revision;
        binding.proof_receipts_sha256 = response_proof_receipts_digest(binding)
            .map_err(|_| "online_admission_proof_receipts_digest_failed")?;
    }
    let registry_sha256 = response_registry_digest(&registry)
        .map_err(|_| "online_admission_registry_digest_failed")?;
    admission.response_authority.registry_sha256 = registry_sha256;
    admission.response_authority.registry_revision = registry.revision;
    if admission.response_authority.packages.len() != registry.packages.len() {
        return Err("online_admission_package_binding_count_mismatch");
    }
    Ok(Some(OnlineAdmissionSnapshot {
        registry,
        admission,
    }))
}

fn authority_content_revision(
    registry: &ResponseRegistry,
    bindings: &[ResponsePackageAuthorityBindingV2],
) -> Result<u64, &'static str> {
    let mut normalized_registry = registry.clone();
    normalized_registry.revision = 0;
    let mut normalized_bindings = bindings.to_vec();
    for binding in &mut normalized_bindings {
        binding.registry_revision = 0;
        binding.proof_receipts_sha256.clear();
    }
    normalized_bindings.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    let digest = canonical_json_sha256(&(
        "nando.response-authority-content-revision.v1",
        normalized_registry,
        normalized_bindings,
    ))?;
    u64::from_str_radix(&digest[..16], 16)
        .map(|revision| revision.max(1))
        .map_err(|_| "online_admission_content_revision_invalid")
}

#[derive(Serialize)]
struct FrameReceiptSet<'a> {
    schema: &'static str,
    package_id: &'a str,
    frame_ids: Vec<&'a str>,
    wrong_accepts: usize,
}

#[derive(Serialize)]
struct RuntimeParityReceipt {
    evidence_ref_sha256: String,
    provider_payload_sha256: String,
    expected_response_sha256: String,
    actual_response_sha256: Option<String>,
    actor_executed: bool,
    independent_verifier_pass: bool,
    exact_response_match: bool,
    execution_budget_normalized_match: bool,
}

#[derive(Serialize)]
struct RuntimeParityReceiptSet<'a> {
    schema: &'static str,
    package_id: &'a str,
    receipts: Vec<RuntimeParityReceipt>,
    failures: usize,
}

#[derive(Serialize)]
struct DurableRuntimeParityReceiptMaterial<'a> {
    schema: &'a str,
    evidence_ref_sha256: &'a str,
    program_sha256: &'a str,
    verifier_sha256: &'a str,
    input_sha256: &'a str,
    teacher_response_sha256: &'a str,
    actor_response_sha256: &'a str,
    actor_executed: bool,
    teacher_authority_match: bool,
    independent_verifier_pass: bool,
    exact_teacher_match: bool,
}

fn durable_runtime_parity_receipt_digest(
    receipt: &DurableRuntimeParityReceipt,
) -> Result<String, &'static str> {
    canonical_json_sha256(&DurableRuntimeParityReceiptMaterial {
        schema: &receipt.schema,
        evidence_ref_sha256: &receipt.evidence_ref_sha256,
        program_sha256: &receipt.program_sha256,
        verifier_sha256: &receipt.verifier_sha256,
        input_sha256: &receipt.input_sha256,
        teacher_response_sha256: &receipt.teacher_response_sha256,
        actor_response_sha256: &receipt.actor_response_sha256,
        actor_executed: receipt.actor_executed,
        teacher_authority_match: receipt.teacher_authority_match,
        independent_verifier_pass: receipt.independent_verifier_pass,
        exact_teacher_match: receipt.exact_teacher_match,
    })
}

pub fn build_durable_runtime_parity_receipt(
    program: &ResponseProgram,
    evidence_ref_sha256: &str,
    example: &CollectionSynthesisExample,
) -> Result<DurableRuntimeParityReceipt, &'static str> {
    if !valid_nonzero_sha256(evidence_ref_sha256) {
        return Err("durable_runtime_parity_evidence_ref_invalid");
    }
    let verifier = source_neutral_verifier_for_program(program)?;
    let execution = execute_response(program, "", &example.provider_payload);
    let actor_executed = execution.status == ResponseExecutionStatus::Executed;
    let actor_response = execution
        .response
        .as_deref()
        .ok_or("durable_runtime_parity_actor_abstained")?;
    let teacher_authority_match = response_program_authority_matches_example(program, example);
    let independent_verifier_pass =
        verify_response_independently(&verifier, &example.provider_payload, actor_response).is_ok();
    if !actor_executed || !teacher_authority_match || !independent_verifier_pass {
        return Err("durable_runtime_parity_verification_failed");
    }
    let mut receipt = DurableRuntimeParityReceipt {
        schema: DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1.to_owned(),
        receipt_sha256: String::new(),
        evidence_ref_sha256: evidence_ref_sha256.to_owned(),
        program_sha256: canonical_json_sha256(program)?,
        verifier_sha256: canonical_json_sha256(&verifier)?,
        input_sha256: canonical_json_sha256(&example.provider_payload)?,
        teacher_response_sha256: sha256_bytes(example.expected_response.as_bytes()),
        actor_response_sha256: sha256_bytes(actor_response.as_bytes()),
        actor_executed,
        teacher_authority_match,
        independent_verifier_pass,
        exact_teacher_match: actor_response == example.expected_response,
    };
    receipt.receipt_sha256 = durable_runtime_parity_receipt_digest(&receipt)?;
    Ok(receipt)
}

fn validate_durable_runtime_parity_receipt(
    receipt: &DurableRuntimeParityReceipt,
    expected_program_sha256: &str,
    expected_verifier_sha256: &str,
) -> bool {
    receipt.schema == DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1
        && valid_nonzero_sha256(&receipt.receipt_sha256)
        && valid_nonzero_sha256(&receipt.evidence_ref_sha256)
        && receipt.program_sha256 == expected_program_sha256
        && receipt.verifier_sha256 == expected_verifier_sha256
        && valid_nonzero_sha256(&receipt.input_sha256)
        && valid_nonzero_sha256(&receipt.teacher_response_sha256)
        && valid_nonzero_sha256(&receipt.actor_response_sha256)
        && receipt.actor_executed
        && receipt.teacher_authority_match
        && receipt.independent_verifier_pass
        && receipt.exact_teacher_match
        && durable_runtime_parity_receipt_digest(receipt)
            .is_ok_and(|digest| digest == receipt.receipt_sha256)
}

fn validate_durable_runtime_parity(
    package: &ResponsePackage,
    durable_receipts: &[DurableRuntimeParityReceipt],
    allowed_evidence_refs: &BTreeSet<&str>,
) -> Result<Option<String>, &'static str> {
    let verifier = package
        .verifier
        .as_ref()
        .ok_or("runtime_parity_verifier_missing")?;
    let program_sha256 = canonical_json_sha256(&package.program)?;
    let verifier_sha256 = canonical_json_sha256(verifier)?;
    let mut seen = BTreeSet::new();
    let mut receipts = Vec::new();
    for receipt in durable_receipts {
        if !allowed_evidence_refs.contains(receipt.evidence_ref_sha256.as_str())
            || !seen.insert(receipt.evidence_ref_sha256.as_str())
        {
            continue;
        }
        if !validate_durable_runtime_parity_receipt(receipt, &program_sha256, &verifier_sha256) {
            return Ok(None);
        }
        receipts.push(RuntimeParityReceipt {
            evidence_ref_sha256: receipt.evidence_ref_sha256.clone(),
            provider_payload_sha256: receipt.input_sha256.clone(),
            expected_response_sha256: receipt.actor_response_sha256.clone(),
            actual_response_sha256: Some(receipt.actor_response_sha256.clone()),
            actor_executed: true,
            independent_verifier_pass: true,
            exact_response_match: true,
            execution_budget_normalized_match: false,
        });
    }
    if receipts.len() < 32 {
        return Ok(None);
    }
    canonical_json_sha256(&RuntimeParityReceiptSet {
        schema: RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1,
        package_id: &package.package_id,
        receipts,
        failures: 0,
    })
    .map(Some)
}

fn execute_runtime_parity(
    package: &ResponsePackage,
    cases: &[crate::RuntimeParityCase],
    allowed_evidence_refs: &BTreeSet<&str>,
) -> Result<Option<String>, &'static str> {
    let verifier = package
        .verifier
        .as_ref()
        .ok_or("runtime_parity_verifier_missing")?;
    let mut seen = BTreeSet::new();
    let mut receipts = Vec::new();
    let mut first_mismatch = None;
    for case in cases {
        if !allowed_evidence_refs.contains(case.evidence_ref_sha256.as_str())
            || !seen.insert(case.evidence_ref_sha256.as_str())
        {
            continue;
        }
        let execution =
            crate::execute_response(&package.program, &case.request_text, &case.provider_payload);
        let actual = execution.response.as_deref();
        let actor_executed = execution.status == crate::ResponseExecutionStatus::Executed;
        let independent_verifier_pass = actual.is_some_and(|response| {
            verify_response_independently(verifier, &case.provider_payload, response).is_ok()
        });
        let exact_response_match = actual == Some(case.expected_response.as_str());
        let execution_budget_normalized_match = actual.is_some_and(|actual| {
            responses_match_after_execution_budget_normalization(actual, &case.expected_response)
        });
        if !exact_response_match && first_mismatch.is_none() {
            first_mismatch = Some(parity_response_diff(actual, &case.expected_response));
        }
        receipts.push(RuntimeParityReceipt {
            evidence_ref_sha256: case.evidence_ref_sha256.clone(),
            provider_payload_sha256: crate::canonical_json_sha256(&case.provider_payload)?,
            expected_response_sha256: crate::sha256_bytes(case.expected_response.as_bytes()),
            actual_response_sha256: actual.map(|response| crate::sha256_bytes(response.as_bytes())),
            actor_executed,
            independent_verifier_pass,
            exact_response_match,
            execution_budget_normalized_match,
        });
    }
    let failures = receipts
        .iter()
        .filter(|receipt| {
            !receipt.actor_executed
                || !receipt.independent_verifier_pass
                || (!receipt.exact_response_match && !receipt.execution_budget_normalized_match)
        })
        .count();
    if std::env::var_os("NANDO_ONLINE_ADMISSION_TRACE").is_some() {
        eprintln!(
            "online_admission parity cases={} allowed_refs={} receipts={} actor_failures={} verifier_failures={} exact_mismatches={} budget_normalized_matches={}",
            cases.len(),
            allowed_evidence_refs.len(),
            receipts.len(),
            receipts
                .iter()
                .filter(|receipt| !receipt.actor_executed)
                .count(),
            receipts
                .iter()
                .filter(|receipt| !receipt.independent_verifier_pass)
                .count(),
            receipts
                .iter()
                .filter(|receipt| !receipt.exact_response_match)
                .count(),
            receipts
                .iter()
                .filter(|receipt| receipt.execution_budget_normalized_match)
                .count(),
        );
        if let Some(mismatch) = first_mismatch {
            eprintln!(
                "online_admission parity_first_mismatch={}",
                serde_json::to_string(&mismatch).unwrap_or_default()
            );
        }
    }
    if receipts.len() < 32 {
        return Ok(None);
    }
    if failures != 0 {
        return Ok(None);
    }
    canonical_json_sha256(&RuntimeParityReceiptSet {
        schema: RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1,
        package_id: &package.package_id,
        receipts,
        failures,
    })
    .map(Some)
}

pub(crate) fn responses_match_after_execution_budget_normalization(
    actual: &str,
    expected: &str,
) -> bool {
    let (Ok(mut actual), Ok(mut expected)) = (
        serde_json::from_str::<serde_json::Value>(actual),
        serde_json::from_str::<serde_json::Value>(expected),
    ) else {
        return false;
    };
    for value in [&mut actual, &mut expected] {
        let Some(arguments) = value
            .get_mut("arguments")
            .and_then(serde_json::Value::as_object_mut)
        else {
            return false;
        };
        arguments.retain(|name, _| !crate::teacher_join::is_execution_budget_argument(name));
    }
    actual == expected
}

fn parity_response_diff(actual: Option<&str>, expected: &str) -> serde_json::Value {
    let actual = actual.and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok());
    let expected = serde_json::from_str::<serde_json::Value>(expected).ok();
    let actual_arguments = actual
        .as_ref()
        .and_then(|value| value.get("arguments"))
        .and_then(serde_json::Value::as_object);
    let expected_arguments = expected
        .as_ref()
        .and_then(|value| value.get("arguments"))
        .and_then(serde_json::Value::as_object);
    let keys = actual_arguments
        .into_iter()
        .flat_map(|arguments| arguments.keys())
        .chain(
            expected_arguments
                .into_iter()
                .flat_map(|arguments| arguments.keys()),
        )
        .cloned()
        .collect::<BTreeSet<_>>();
    let differences = keys
        .into_iter()
        .filter_map(|key| {
            let actual = actual_arguments.and_then(|arguments| arguments.get(&key));
            let expected = expected_arguments.and_then(|arguments| arguments.get(&key));
            (actual != expected).then(|| {
                let value = match (actual, expected) {
                    (Some(serde_json::Value::Number(actual)), Some(serde_json::Value::Number(expected))) => {
                        serde_json::json!({"kind":"number", "actual":actual, "expected":expected})
                    }
                    (Some(serde_json::Value::Bool(actual)), Some(serde_json::Value::Bool(expected))) => {
                        serde_json::json!({"kind":"boolean", "actual":actual, "expected":expected})
                    }
                    (Some(serde_json::Value::String(actual)), Some(serde_json::Value::String(expected))) => {
                        serde_json::json!({"kind":"string", "actual_bytes":actual.len(), "expected_bytes":expected.len()})
                    }
                    (actual, expected) => serde_json::json!({
                        "kind":"shape",
                        "actual": actual.map(parity_value_kind),
                        "expected": expected.map(parity_value_kind),
                    }),
                };
                (key, value)
            })
        })
        .collect::<BTreeMap<_, _>>();
    serde_json::json!({
        "actual_name": actual.as_ref().and_then(|value| value.get("name")).and_then(serde_json::Value::as_str),
        "expected_name": expected.as_ref().and_then(|value| value.get("name")).and_then(serde_json::Value::as_str),
        "differences": differences,
    })
}

fn parity_value_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn program_required_client_capability_atom(program: &crate::ResponseProgram) -> Option<u64> {
    match &program.operation {
        crate::ResponseOperation::AdvancePlan { function_name } => Some(
            crate::package::stable_atom_id(&format!("client_capability:function:{function_name}")),
        ),
        crate::ResponseOperation::FunctionCallFromRoles {
            function_name,
            selector,
            ..
        } if !matches!(
            selector,
            crate::ResponseValueSelector::ContentLinePrefix { prefix, .. }
                if prefix == "Process running with session ID "
        ) =>
        {
            Some(crate::package::stable_atom_id(&format!(
                "client_capability:function:{function_name}"
            )))
        }
        crate::ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name, ..
        } => Some(crate::package::stable_atom_id(&format!(
            "client_capability:custom:{custom_tool_name}"
        ))),
        _ => None,
    }
}

fn trace_online_admission(candidate: &OnlineResponseAdmissionCandidate, blocker: &str) {
    if std::env::var_os("NANDO_ONLINE_ADMISSION_TRACE").is_some() {
        eprintln!(
            "online_admission bucket={} blocker={blocker}",
            candidate.candidate.bucket_id
        );
    }
}

fn bind_proven_semantic_law_program(
    package: &mut crate::ResponsePackage,
    program: &crate::ResponseProgram,
    training: &[crate::RelationFrame],
) -> Result<(), &'static str> {
    if !matches!(
        program.operation,
        crate::ResponseOperation::UniqueConsensus { .. }
    ) {
        return Ok(());
    }
    program
        .validate()
        .map_err(|_| "semantic_law_program_invalid")?;
    if !training
        .iter()
        .all(|frame| crate::synthesis::program_is_consistent(program, frame))
    {
        return Err("semantic_law_support_mismatch");
    }
    let verifier = crate::source_neutral_verifier_for_program(program)
        .map_err(|_| "semantic_law_verifier_missing")?;
    let verifier_schema = crate::response_program_external_verifier_schema(program)
        .ok_or("semantic_law_verifier_schema_missing")?;
    package.program = program.clone();
    package.verifier = Some(verifier);
    package.proof.verifier_schema = verifier_schema.to_owned();
    Ok(())
}

pub fn build_online_admission_snapshot(
    candidates: &[OnlineResponseAdmissionCandidate],
    project_id: &str,
    revision: u64,
    now_unix: u64,
    max_age_seconds: u64,
    gate_build_sha256: &str,
    runtime_build_sha256: &str,
) -> Result<Option<OnlineAdmissionSnapshot>, &'static str> {
    let mut packages = Vec::new();
    let mut receipt_digests = BTreeMap::new();
    for candidate in candidates {
        if candidate.support.len() < 32 || candidate.future.len() < 32 {
            trace_online_admission(
                candidate,
                &format!(
                    "rows_below_32 support={} future={}",
                    candidate.support.len(),
                    candidate.future.len()
                ),
            );
            continue;
        }
        let support_frame_ids = candidate
            .support
            .iter()
            .map(|frame| frame.frame_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let support_event_ids = candidate
            .support
            .iter()
            .map(|frame| frame.event_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        if candidate.future.iter().any(|frame| {
            support_frame_ids.contains(frame.frame_id_sha256.as_str())
                || support_event_ids.contains(frame.event_id_sha256.as_str())
        }) {
            trace_online_admission(candidate, "support_future_overlap");
            continue;
        }
        let parity_by_ref = candidate
            .runtime_parity_cases
            .iter()
            .map(|case| (case.evidence_ref_sha256.as_str(), case))
            .collect::<BTreeMap<_, _>>();
        let training = candidate
            .support
            .iter()
            .map(|frame| {
                action_schema_enriched_frame(
                    frame,
                    parity_by_ref.get(frame.frame_id_sha256.as_str()).copied(),
                )
            })
            .collect::<Vec<_>>();
        let Some(mut package) = compile_source_neutral_quarantine_packages(&training, true)
            .into_iter()
            .next()
        else {
            trace_online_admission(candidate, "package_compile_empty");
            continue;
        };
        if let Err(blocker) =
            bind_proven_semantic_law_program(&mut package, &candidate.candidate.program, &training)
        {
            trace_online_admission(candidate, blocker);
            continue;
        }
        // The streaming subcenter already supplies an action-neutral exact
        // guard proven clean against global negatives. Reapplying the legacy
        // broad-family cardinality refinement here destroys cross-layout
        // coverage without adding authority-path evidence.
        package.routing_predicates.clear();
        package.required_routing_atom_ids =
            response_program_required_routing_atom_ids(&package.program);
        package
            .required_routing_atom_ids
            .extend(candidate.required_routing_atom_ids.iter().copied());
        package.required_routing_atom_ids.sort_unstable();
        package.required_routing_atom_ids.dedup();
        let Some(learned_wave_route) = learned_wave_route_from_bytes(
            &candidate.wave_runtime_package,
            candidate.candidate.wave_threshold_micro,
        ) else {
            trace_online_admission(candidate, "learned_wave_route_invalid");
            continue;
        };
        package.wave_margin_micro = learned_wave_route.threshold_micro;
        package.learned_wave_route = Some(LearnedWaveRoute {
            query_atom_ids: Vec::new(),
            ..learned_wave_route
        });
        let lineage = crate::response_package_lineage_id(
            &package.program,
            &package.required_routing_atom_ids,
        );
        let subcenter_lineage = crate::sha256_bytes(
            &serde_json::to_vec(&(
                "nando.response-subcenter-lineage.v1",
                lineage,
                &candidate.required_routing_atom_ids,
            ))
            .unwrap_or_default(),
        );
        package.package_id = crate::grounded_response_package_id(&subcenter_lineage, 0);
        let mut refined_support = training
            .iter()
            .filter(|frame| crate::package::relation_frame_matches_package_guard(&package, frame))
            .cloned()
            .collect::<Vec<_>>();
        if refined_support.len() < 32 {
            trace_online_admission(
                candidate,
                &format!("refined_support_below_32 rows={}", refined_support.len()),
            );
            continue;
        }
        let guard_relevant_negatives = candidate
            .negatives
            .iter()
            .filter(|frame| crate::package::relation_frame_matches_package_guard(&package, frame))
            .cloned()
            .collect::<Vec<_>>();
        if !ensure_support_separating_learned_route(
            &mut package,
            &refined_support,
            &guard_relevant_negatives,
        ) {
            trace_online_admission(
                candidate,
                &format!(
                    "route_threshold_overlap guard_relevant_negatives={}",
                    guard_relevant_negatives.len()
                ),
            );
            continue;
        }
        let program_future = candidate
            .future
            .iter()
            .map(|frame| {
                action_schema_enriched_frame(
                    frame,
                    parity_by_ref.get(frame.frame_id_sha256.as_str()).copied(),
                )
            })
            .filter(|frame| {
                frame_matches_program_action_contract(&package.program, frame)
                    && program_required_client_capability_atom(&package.program).is_none_or(
                        |required| {
                            frame.atoms.iter().any(|atom| {
                                matches!(
                                    atom,
                                    crate::RelationAtom::ClientCapabilityAtom { atom_id }
                                        | crate::RelationAtom::ReconstructedClientCapabilityAtom { atom_id }
                                        if *atom_id == required
                                )
                            })
                        },
                    )
            })
            .collect::<Vec<_>>();
        if std::env::var_os("NANDO_ONLINE_ADMISSION_TRACE").is_some() {
            let required_coverage = package
                .required_routing_atom_ids
                .iter()
                .map(|required| {
                    let rows = program_future
                        .iter()
                        .filter(|frame| {
                            crate::relation_frame_online_routing_atom_ids(frame)
                                .binary_search(required)
                                .is_ok()
                        })
                        .count();
                    (*required, rows)
                })
                .collect::<Vec<_>>();
            let negative_required_coverage = package
                .required_routing_atom_ids
                .iter()
                .map(|required| {
                    let rows = candidate
                        .negatives
                        .iter()
                        .filter(|frame| {
                            crate::relation_frame_online_routing_atom_ids(frame)
                                .binary_search(required)
                                .is_ok()
                        })
                        .count();
                    (*required, rows)
                })
                .collect::<Vec<_>>();
            let negative_full_guard = candidate
                .negatives
                .iter()
                .filter(|frame| {
                    crate::package::relation_frame_matches_package_guard(&package, frame)
                })
                .count();
            let support_margins = refined_support
                .iter()
                .filter_map(|frame| crate::relation_frame_phase_margin_micro(&package, frame))
                .collect::<Vec<_>>();
            let negative_margins = candidate
                .negatives
                .iter()
                .filter_map(|frame| crate::relation_frame_phase_margin_micro(&package, frame))
                .collect::<Vec<_>>();
            let support_by_vector = refined_support
                .iter()
                .map(|frame| (crate::relation_frame_online_routing_atom_ids(frame), frame))
                .collect::<BTreeMap<_, _>>();
            let exact_vector_collisions = candidate
                .negatives
                .iter()
                .filter_map(|negative| {
                    support_by_vector
                        .get(&crate::relation_frame_online_routing_atom_ids(negative))
                        .map(|positive| {
                            (
                                positive.frame_id_sha256.as_str(),
                                negative.frame_id_sha256.as_str(),
                                negative.observed_at_unix_nanos,
                            )
                        })
                })
                .collect::<Vec<_>>();
            let collision_atom_diffs = candidate
                .negatives
                .iter()
                .filter_map(|negative| {
                    support_by_vector
                        .get(&crate::relation_frame_online_routing_atom_ids(negative))
                        .map(|positive| {
                            serde_json::json!({
                                "positive": positive.frame_id_sha256,
                                "negative": negative.frame_id_sha256,
                                "common_atoms": positive.atoms.iter().filter(|atom| negative.atoms.contains(atom)).collect::<Vec<_>>(),
                                "positive_only": positive.atoms.iter().filter(|atom| !negative.atoms.contains(atom)).collect::<Vec<_>>(),
                                "negative_only": negative.atoms.iter().filter(|atom| !positive.atoms.contains(atom)).collect::<Vec<_>>(),
                            })
                        })
                })
                .collect::<Vec<_>>();
            let clean_guard_candidates = clean_guard_candidates(
                &refined_support.iter().collect::<Vec<_>>(),
                &program_future.iter().collect::<Vec<_>>(),
                &candidate.negatives.iter().collect::<Vec<_>>(),
                32,
                8,
            );
            let best_phase_medoid = best_phase_medoid_coverage(
                &refined_support.iter().collect::<Vec<_>>(),
                &program_future.iter().collect::<Vec<_>>(),
                &candidate.negatives.iter().collect::<Vec<_>>(),
                32,
            );
            eprintln!(
                "online_admission bucket={} program_future={} required_coverage={required_coverage:?} negative_required_coverage={negative_required_coverage:?} negative_full_guard={negative_full_guard} support_margin_min={:?} support_margin_max={:?} negative_margin_max={:?} imported_threshold={} exact_vector_collisions={exact_vector_collisions:?} negative_ids={:?} clean_guard_candidates={clean_guard_candidates:?} best_phase_medoid={best_phase_medoid:?} collision_atom_diffs={}",
                candidate.candidate.bucket_id,
                program_future.len(),
                support_margins.iter().min(),
                support_margins.iter().max(),
                negative_margins.iter().max(),
                package.wave_margin_micro,
                candidate
                    .negatives
                    .iter()
                    .map(|frame| frame.frame_id_sha256.as_str())
                    .collect::<Vec<_>>(),
                serde_json::to_string(&collision_atom_diffs).unwrap_or_default(),
            );
        }
        let (phase_support, phase_future) = phase_clean_support_future(
            &mut package,
            &refined_support,
            &program_future,
            &candidate.negatives,
        );
        refined_support = phase_support;
        if refined_support.len() < 32 || phase_future.len() < 32 {
            trace_online_admission(
                candidate,
                &format!(
                    "phase_clean_rows_below_32 support={} future={}",
                    refined_support.len(),
                    phase_future.len()
                ),
            );
            continue;
        }
        let routed_future = phase_future
            .iter()
            .filter(|frame| relation_frame_routes_to_package(&package, frame))
            .cloned()
            .collect::<Vec<_>>();
        if routed_future.len() < 32 {
            trace_online_admission(
                candidate,
                &format!("routed_future_below_32 rows={}", routed_future.len()),
            );
            continue;
        }
        let future_wrong = routed_future
            .iter()
            .filter(|frame| {
                frame.verifier_label != Some(true)
                    || !frame_matches_program_action_contract(&package.program, frame)
            })
            .count();
        let future_sessions = routed_future
            .iter()
            .map(|frame| frame.session_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let surfaces = refined_support
            .iter()
            .chain(routed_future.iter())
            .filter_map(relation_frame_structural_family_id)
            .collect::<BTreeSet<_>>();
        if future_sessions.len() < 3 || surfaces.len() < 2 {
            trace_online_admission(
                candidate,
                &format!(
                    "diversity_below_gate sessions={} surfaces={}",
                    future_sessions.len(),
                    surfaces.len()
                ),
            );
            continue;
        }
        let negative_accepts = candidate
            .negatives
            .iter()
            .filter(|frame| relation_frame_routes_to_package(&package, frame))
            .count();
        let causal = evaluate_grounded_wave_causality(
            &package,
            &refined_support,
            &routed_future,
            &candidate.negatives,
        );
        if future_wrong != 0 || negative_accepts != 0 || causal.verdict != "PASS" {
            trace_online_admission(
                candidate,
                &format!(
                    "proof_failed future_wrong={future_wrong} negative_accepts={negative_accepts} causal={} full={}/{} shuffled={} random={} ablation_negative_accepts={}/{} margins={}/{}/{}",
                    causal.verdict,
                    causal.full_phase_correct,
                    causal.future_rows,
                    causal.shuffled_phase_correct,
                    causal.random_center_correct,
                    causal.shuffled_negative_accepts,
                    causal.random_center_negative_accepts,
                    causal.full_margin_mean_micro,
                    causal.shuffled_margin_mean_micro,
                    causal.random_margin_mean_micro,
                ),
            );
            continue;
        }
        trace_online_admission(candidate, "PASS");
        package.state = ResponsePackageState::Active;
        package.proof.support_rows = refined_support.len();
        package.proof.future_rows = routed_future.len();
        package.proof.distinct_sessions = future_sessions.len();
        package.proof.distinct_surfaces = surfaces.len();
        package.proof.wrong_accepts = 0;
        let routed_future_refs = routed_future
            .iter()
            .flat_map(|frame| {
                [
                    frame.frame_id_sha256.as_str(),
                    frame.evidence_ref_sha256.as_str(),
                ]
            })
            .collect::<BTreeSet<_>>();
        let Some(parity_sha256) = execute_runtime_parity(
            &package,
            &candidate.runtime_parity_cases,
            &routed_future_refs,
        )?
        else {
            trace_online_admission(candidate, "runtime_parity_cases_below_gate_or_failed");
            continue;
        };
        package.proof.runtime_parity_failures = 0;
        package.proof.exact_cache_overlap = 0;
        package.proof.wave_causal_pass = true;
        if !package.eligible_for_admission_candidate() {
            trace_online_admission(
                candidate,
                package
                    .admission_candidate_blocker()
                    .unwrap_or("package_not_eligible"),
            );
            continue;
        }
        let support_sha256 = canonical_json_sha256(&FrameReceiptSet {
            schema: RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1,
            package_id: &package.package_id,
            frame_ids: refined_support
                .iter()
                .map(|frame| frame.frame_id_sha256.as_str())
                .collect(),
            wrong_accepts: 0,
        })?;
        let causal_sha256 = canonical_json_sha256(&causal)?;
        let future_sha256 = canonical_json_sha256(&FrameReceiptSet {
            schema: RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2,
            package_id: &package.package_id,
            frame_ids: routed_future
                .iter()
                .map(|frame| frame.evidence_ref_sha256.as_str())
                .collect(),
            wrong_accepts: 0,
        })?;
        let semantic_alias_sha256 = semantic_alias_future_proof_digest(
            candidate,
            &refined_support,
            &routed_future,
            &causal,
        )?;
        receipt_digests.insert(
            package.package_id.clone(),
            (
                support_sha256,
                causal_sha256,
                parity_sha256,
                future_sha256,
                semantic_alias_sha256,
            ),
        );
        packages.push(package);
    }
    if packages.is_empty() {
        return Ok(None);
    }
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    packages.dedup_by(|left, right| left.package_id == right.package_id);
    let registry = ResponseRegistry {
        schema: RESPONSE_REGISTRY_SCHEMA_V6.to_owned(),
        revision,
        packages,
    };
    registry.validate()?;
    let registry_sha256 = response_registry_digest(&registry)?;
    let mut bindings = Vec::new();
    for package in &registry.packages {
        let (support, causal, parity, future, semantic_alias) = receipt_digests
            .remove(&package.package_id)
            .ok_or("online_admission_receipts_missing")?;
        let verifier = package
            .verifier
            .as_ref()
            .ok_or("online_admission_verifier_missing")?;
        let mut binding = ResponsePackageAuthorityBindingV2 {
            package_id: package.package_id.clone(),
            registry_revision: revision,
            package_sha256: response_package_digest(package)?,
            execution_payload_sha256: response_execution_payload_digest(package)?,
            actor_program_sha256: response_actor_program_digest(&package.program)?,
            independent_verifier_program_sha256: response_independent_verifier_program_digest(
                verifier,
            )?,
            verifier_schema: package.proof.verifier_schema.clone(),
            support_manifest_schema: RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1.to_owned(),
            support_manifest_sha256: support,
            exact_causal_proof_schema: RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2.to_owned(),
            exact_causal_proof_sha256: causal,
            runtime_parity_receipt_set_schema: RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1
                .to_owned(),
            runtime_parity_receipt_set_sha256: parity,
            future_verifier_receipt_set_schema: RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2
                .to_owned(),
            future_verifier_receipt_set_sha256: future,
            semantic_alias_proof_schema: RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1.to_owned(),
            semantic_alias_proof_sha256: semantic_alias,
            proof_receipts_sha256: String::new(),
        };
        binding.proof_receipts_sha256 = response_proof_receipts_digest(&binding)?;
        bindings.push(binding);
    }
    bindings.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    let admission = CompositeResponseAdmissionV2 {
        schema: COMPOSITE_ADMISSION_SCHEMA_V2.to_owned(),
        project_id: project_id.to_owned(),
        generated_at_unix: now_unix,
        expires_at_unix: now_unix.saturating_add(max_age_seconds),
        verdict: "PASS".to_owned(),
        eligible_for_local_accept: true,
        response_authority: ResponseAuthorityV2 {
            schema: RESPONSE_AUTHORITY_SCHEMA_V2.to_owned(),
            registry_schema: registry.schema.clone(),
            registry_revision: revision,
            registry_sha256,
            gate_build_sha256: gate_build_sha256.to_owned(),
            runtime_build_sha256: runtime_build_sha256.to_owned(),
            packages: bindings,
        },
    };
    Ok(Some(OnlineAdmissionSnapshot {
        registry,
        admission,
    }))
}

fn semantic_alias_future_proof_digest(
    candidate: &OnlineResponseAdmissionCandidate,
    refined_support: &[crate::RelationFrame],
    routed_future: &[crate::RelationFrame],
    causal: &crate::GroundedWaveCausalReport,
) -> Result<String, &'static str> {
    let physical_signatures = candidate
        .support
        .iter()
        .filter_map(crate::teacher_program_signature)
        .collect::<BTreeSet<_>>();
    if candidate.semantic_alias_edges.is_empty() {
        if physical_signatures.len() > 1 {
            return Err("semantic_alias_proof_missing_for_multi_adapter_candidate");
        }
        return canonical_json_sha256(&(
            RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1,
            "exact_singleton",
            candidate.candidate.teacher_signature_sha256.as_str(),
            &candidate.candidate.program,
            causal,
        ));
    }

    let support_ids = candidate
        .support
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let parity_receipts = candidate
        .runtime_parity_cases
        .iter()
        .map(|receipt| receipt.evidence_ref_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let future_receipts = routed_future
        .iter()
        .flat_map(|frame| {
            [
                frame.frame_id_sha256.as_str(),
                frame.evidence_ref_sha256.as_str(),
            ]
        })
        .collect::<BTreeSet<_>>();
    for edge in &candidate.semantic_alias_edges {
        if !matches!(
            edge.state,
            crate::SemanticAliasState::SupportProven | crate::SemanticAliasState::FutureProven
        ) || edge.effect_graph_sha256 != candidate.candidate.teacher_signature_sha256
            || !physical_signatures.contains(&edge.left_teacher_signature_sha256)
            || !physical_signatures.contains(&edge.right_teacher_signature_sha256)
            || edge.support_receipts.is_empty()
            || edge
                .support_receipts
                .iter()
                .any(|receipt| !support_ids.contains(receipt.as_str()))
            || edge.parity_receipts.is_empty()
            || edge
                .parity_receipts
                .iter()
                .any(|receipt| !parity_receipts.contains(receipt.as_str()))
            || edge
                .wave_proof_sha256
                .as_deref()
                .is_none_or(|digest| !valid_nonzero_sha256(digest))
            || edge.blocker.is_some()
            || !edge.counterexamples.is_empty()
        {
            return Err("semantic_alias_support_proof_invalid");
        }
        if edge.state == crate::SemanticAliasState::FutureProven
            && (edge.future_receipts.is_empty()
                || edge
                    .future_receipts
                    .iter()
                    .any(|receipt| !future_receipts.contains(receipt.as_str())))
        {
            return Err("semantic_alias_future_receipt_mismatch");
        }
    }
    let refined_support_ids = refined_support
        .iter()
        .map(|frame| frame.frame_id_sha256.as_str())
        .collect::<Vec<_>>();
    let routed_future_ids = routed_future
        .iter()
        .map(|frame| frame.evidence_ref_sha256.as_str())
        .collect::<Vec<_>>();
    let mut future_proven_edges = candidate.semantic_alias_edges.clone();
    for edge in &mut future_proven_edges {
        edge.state = crate::SemanticAliasState::FutureProven;
        edge.future_receipts = routed_future_ids
            .iter()
            .map(|value| (*value).to_owned())
            .collect();
        edge.blocker = None;
    }
    canonical_json_sha256(&(
        RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1,
        "future_proven",
        0_u64,
        future_proven_edges,
        refined_support_ids,
        routed_future_ids,
        causal,
    ))
}

fn clean_guard_candidates(
    support: &[&crate::RelationFrame],
    future: &[&crate::RelationFrame],
    negatives: &[&crate::RelationFrame],
    minimum_rows: usize,
    limit: usize,
) -> Vec<(Vec<u64>, usize, usize)> {
    let support_atoms = support
        .iter()
        .map(|frame| crate::relation_frame_online_routing_atom_ids(frame))
        .collect::<Vec<_>>();
    let future_atoms = future
        .iter()
        .map(|frame| crate::relation_frame_online_routing_atom_ids(frame))
        .collect::<Vec<_>>();
    let negative_atoms = negatives
        .iter()
        .map(|frame| crate::relation_frame_online_routing_atom_ids(frame))
        .collect::<Vec<_>>();
    let mut atom_rows = BTreeMap::<u64, usize>::new();
    for observed in support_atoms.iter().chain(&future_atoms) {
        for atom in observed {
            *atom_rows.entry(*atom).or_default() += 1;
        }
    }
    let mut atoms = atom_rows.into_iter().collect::<Vec<_>>();
    atoms.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    atoms.truncate(64);
    let atom_ids = atoms.iter().map(|(atom, _)| *atom).collect::<Vec<_>>();
    let mut predicates = atom_ids.iter().map(|atom| vec![*atom]).collect::<Vec<_>>();
    for (left_index, left) in atom_ids.iter().enumerate() {
        for right in atom_ids.iter().skip(left_index.saturating_add(1)) {
            predicates.push(vec![*left, *right]);
        }
    }
    let mut candidates = predicates
        .into_iter()
        .filter_map(|predicate| {
            let support_rows = support_atoms
                .iter()
                .filter(|observed| {
                    predicate
                        .iter()
                        .all(|atom| observed.binary_search(atom).is_ok())
                })
                .count();
            let future_rows = future_atoms
                .iter()
                .filter(|observed| {
                    predicate
                        .iter()
                        .all(|atom| observed.binary_search(atom).is_ok())
                })
                .count();
            if support_rows.saturating_add(future_rows) < minimum_rows
                || negative_atoms.iter().any(|observed| {
                    predicate
                        .iter()
                        .all(|atom| observed.binary_search(atom).is_ok())
                })
            {
                return None;
            }
            Some((predicate, support_rows, future_rows))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .1
            .saturating_add(right.2)
            .cmp(&left.1.saturating_add(left.2))
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.0.len().cmp(&right.0.len()))
            .then_with(|| left.0.cmp(&right.0))
    });
    candidates.truncate(limit);
    candidates
}

type PhaseCoverPoint = (usize, usize, usize);
type PhaseMedoidCoverage = (String, usize, usize, i64, Vec<PhaseCoverPoint>);

fn best_phase_medoid_coverage(
    support: &[&crate::RelationFrame],
    future: &[&crate::RelationFrame],
    negatives: &[&crate::RelationFrame],
    cells: usize,
) -> Option<PhaseMedoidCoverage> {
    if support.is_empty() || negatives.is_empty() || cells == 0 {
        return None;
    }
    let support_vectors = support
        .iter()
        .map(|frame| {
            phase_vector_from_atom_ids(crate::relation_frame_online_routing_atom_ids(frame), cells)
        })
        .collect::<Vec<_>>();
    let future_vectors = future
        .iter()
        .map(|frame| {
            phase_vector_from_atom_ids(crate::relation_frame_online_routing_atom_ids(frame), cells)
        })
        .collect::<Vec<_>>();
    let negative_vectors = negatives
        .iter()
        .map(|frame| {
            phase_vector_from_atom_ids(crate::relation_frame_online_routing_atom_ids(frame), cells)
        })
        .collect::<Vec<_>>();
    let mut negative_center = vec![PhaseCenterCell::default(); cells];
    for vector in &negative_vectors {
        for (center, cell) in negative_center.iter_mut().zip(vector) {
            center.re += cell.re / negative_vectors.len() as f64;
            center.im += cell.im / negative_vectors.len() as f64;
        }
    }
    let score = |vector: &[PhaseCenterCell], delta: &[PhaseCenterCell]| {
        let value = vector
            .iter()
            .zip(delta)
            .map(|(query, center)| query.re * center.re + query.im * center.im)
            .sum::<f64>()
            / cells as f64;
        phase_margin_to_micro(value).unwrap_or(i64::MIN)
    };
    let mut medoids = Vec::<(String, i64, Vec<usize>, Vec<usize>)>::new();
    let mut best = None::<(String, usize, usize, i64)>;
    for (index, representative) in support_vectors.iter().enumerate() {
        let delta = representative
            .iter()
            .zip(&negative_center)
            .map(|(positive, negative)| PhaseCenterCell {
                re: positive.re - negative.re,
                im: positive.im - negative.im,
            })
            .collect::<Vec<_>>();
        let threshold = negative_vectors
            .iter()
            .map(|vector| score(vector, &delta))
            .max()
            .unwrap_or(i64::MIN)
            .saturating_add(1)
            .max(1);
        let support_matches = support_vectors
            .iter()
            .enumerate()
            .filter_map(|(index, vector)| (score(vector, &delta) >= threshold).then_some(index))
            .collect::<Vec<_>>();
        let future_matches = future_vectors
            .iter()
            .enumerate()
            .filter_map(|(index, vector)| (score(vector, &delta) >= threshold).then_some(index))
            .collect::<Vec<_>>();
        let candidate = (
            support[index].frame_id_sha256.clone(),
            support_matches.len(),
            future_matches.len(),
            threshold,
        );
        if best.as_ref().is_none_or(|current| {
            (
                candidate.1,
                candidate.2,
                std::cmp::Reverse(candidate.3),
                &candidate.0,
            ) > (
                current.1,
                current.2,
                std::cmp::Reverse(current.3),
                &current.0,
            )
        }) {
            best = Some(candidate.clone());
        }
        medoids.push((candidate.0, threshold, support_matches, future_matches));
    }
    let mut selected = BTreeSet::new();
    let mut covered_support = vec![false; support.len()];
    let mut covered_future = vec![false; future.len()];
    let mut cover_curve = Vec::new();
    for step in 0..8 {
        let next = medoids
            .iter()
            .enumerate()
            .filter(|(index, _)| !selected.contains(index))
            .map(|(index, medoid)| {
                let new_support = medoid
                    .2
                    .iter()
                    .filter(|row| !covered_support[**row])
                    .count();
                let new_future = medoid.3.iter().filter(|row| !covered_future[**row]).count();
                (index, new_support, new_future, medoid.0.as_str())
            })
            .max_by(|left, right| {
                (left.1, left.2, std::cmp::Reverse(left.3)).cmp(&(
                    right.1,
                    right.2,
                    std::cmp::Reverse(right.3),
                ))
            });
        let Some((index, new_support, new_future, _)) = next else {
            break;
        };
        if new_support == 0 && new_future == 0 {
            break;
        }
        selected.insert(index);
        for row in &medoids[index].2 {
            covered_support[*row] = true;
        }
        for row in &medoids[index].3 {
            covered_future[*row] = true;
        }
        cover_curve.push((
            step + 1,
            covered_support.iter().filter(|covered| **covered).count(),
            covered_future.iter().filter(|covered| **covered).count(),
        ));
    }
    best.map(|best| (best.0, best.1, best.2, best.3, cover_curve))
}

pub(crate) fn learned_wave_route_from_bytes(
    bytes: &[u8],
    threshold_micro: i64,
) -> Option<LearnedWaveRoute> {
    let runtime = PhaseCenterFlatRuntime::from_bytes(bytes).ok()?;
    if runtime.record_count() != 1 || runtime.cells() > usize::from(u16::MAX) {
        return None;
    }
    let record = runtime.record(0).ok()?;
    let center_delta_micro = record
        .positive_center
        .iter()
        .zip(record.negative_center.iter())
        .flat_map(|(positive, negative)| {
            [
                ((positive.re - negative.re) * 1_000_000.0).round() as i32,
                ((positive.im - negative.im) * 1_000_000.0).round() as i32,
            ]
        })
        .collect::<Vec<_>>();
    Some(LearnedWaveRoute {
        cells: runtime.cells() as u16,
        center_delta_micro,
        threshold_micro,
        query_atom_ids: Vec::new(),
        subcenters: Vec::new(),
    })
}

pub(crate) fn calibrate_learned_route_threshold(
    package: &mut ResponsePackage,
    support: &[crate::RelationFrame],
    negatives: &[crate::RelationFrame],
) -> bool {
    if package
        .learned_wave_route
        .as_ref()
        .is_some_and(|route| !route.query_atom_ids.is_empty())
    {
        return support
            .iter()
            .filter(|frame| crate::relation_frame_routes_to_package(package, frame))
            .count()
            >= 32
            && negatives
                .iter()
                .all(|frame| !crate::relation_frame_routes_to_package(package, frame));
    }
    let support_margins = support
        .iter()
        .filter_map(|frame| crate::relation_frame_phase_margin_micro(package, frame))
        .collect::<Vec<_>>();
    if support_margins.len() != support.len() || support_margins.is_empty() {
        return false;
    }
    let minimum_positive = support_margins.into_iter().min().unwrap_or(i64::MIN);
    let maximum_negative = negatives
        .iter()
        .filter_map(|frame| crate::relation_frame_phase_margin_micro(package, frame))
        .max()
        .unwrap_or(i64::MIN);
    let threshold = maximum_negative.saturating_add(1).max(1);
    if threshold > minimum_positive {
        return false;
    }
    package.wave_margin_micro = threshold;
    if let Some(route) = package.learned_wave_route.as_mut() {
        route.threshold_micro = threshold;
    }
    true
}

pub(crate) fn ensure_support_separating_learned_route(
    package: &mut ResponsePackage,
    support: &[crate::RelationFrame],
    negatives: &[crate::RelationFrame],
) -> bool {
    // Preserve the proof-carrying center emitted by the streaming miner when
    // it already separates support from negatives. An empty vocabulary means
    // all online routing atoms, matching the miner's training representation;
    // it is not a request to replace the learned center with a medoid.
    if calibrate_learned_route_threshold(package, support, negatives) {
        return true;
    }
    let Some(cells) = package
        .learned_wave_route
        .as_ref()
        .map(|route| usize::from(route.cells))
    else {
        return false;
    };
    let Some(route) = learned_wave_route_from_support_medoid(support, negatives, cells) else {
        return false;
    };
    package.wave_margin_micro = route.threshold_micro;
    package.learned_wave_route = Some(route);
    calibrate_learned_route_threshold(package, support, negatives)
}

pub(crate) fn learned_wave_route_from_support_medoid(
    support: &[crate::RelationFrame],
    negatives: &[crate::RelationFrame],
    cells: usize,
) -> Option<LearnedWaveRoute> {
    if support.len() < 32 || negatives.is_empty() || cells == 0 || cells > usize::from(u16::MAX) {
        return None;
    }
    let query_atom_ids = learned_wave_feature_vocabulary(support, negatives, 256);
    if query_atom_ids.is_empty() {
        return None;
    }
    let filtered_atoms = |frame: &crate::RelationFrame| {
        let mut atoms = crate::relation_frame_online_routing_atom_ids(frame);
        atoms.retain(|atom| query_atom_ids.binary_search(atom).is_ok());
        atoms
    };
    let support_vectors = support
        .iter()
        .map(|frame| phase_vector_from_atom_ids(filtered_atoms(frame), cells))
        .collect::<Vec<_>>();
    let negative_vectors = negatives
        .iter()
        .map(|frame| phase_vector_from_atom_ids(filtered_atoms(frame), cells))
        .collect::<Vec<_>>();
    let mut negative_center = vec![PhaseCenterCell::default(); cells];
    for vector in &negative_vectors {
        for (center, cell) in negative_center.iter_mut().zip(vector) {
            center.re += cell.re / negative_vectors.len() as f64;
            center.im += cell.im / negative_vectors.len() as f64;
        }
    }
    let score = |vector: &[PhaseCenterCell], center_delta_micro: &[i32]| {
        phase_margin_to_micro(
            vector
                .iter()
                .zip(center_delta_micro.chunks_exact(2))
                .map(|(query, center)| {
                    query.re * f64::from(center[0]) / 1_000_000.0
                        + query.im * f64::from(center[1]) / 1_000_000.0
                })
                .sum::<f64>()
                / cells as f64,
        )
        .unwrap_or(i64::MIN)
    };

    #[derive(Clone)]
    struct MedoidCandidate {
        coverage: BTreeSet<usize>,
        gap_micro: i64,
        threshold_micro: i64,
        frame_id: String,
        center_delta_micro: Vec<i32>,
    }

    let mut candidates = Vec::<MedoidCandidate>::new();
    let mut seen_routes = BTreeSet::<(Vec<i32>, i64)>::new();
    for (index, representative) in support_vectors.iter().enumerate() {
        let delta = representative
            .iter()
            .zip(&negative_center)
            .map(|(positive, negative)| PhaseCenterCell {
                re: positive.re - negative.re,
                im: positive.im - negative.im,
            })
            .collect::<Vec<_>>();
        let center_delta_micro = delta
            .into_iter()
            .flat_map(|cell| {
                [
                    (cell.re * 1_000_000.0).round() as i32,
                    (cell.im * 1_000_000.0).round() as i32,
                ]
            })
            .collect::<Vec<_>>();
        let maximum_negative = negative_vectors
            .iter()
            .map(|vector| score(vector, &center_delta_micro))
            .max()
            .unwrap_or(i64::MIN);
        let Some(threshold_micro) = maximum_negative.checked_add(1).map(|value| value.max(1))
        else {
            continue;
        };
        if negative_vectors
            .iter()
            .any(|vector| score(vector, &center_delta_micro) >= threshold_micro)
        {
            continue;
        }
        let support_margins = support_vectors
            .iter()
            .map(|vector| score(vector, &center_delta_micro))
            .collect::<Vec<_>>();
        let coverage = support_margins
            .iter()
            .enumerate()
            .filter_map(|(support_index, margin)| {
                (*margin >= threshold_micro).then_some(support_index)
            })
            .collect::<BTreeSet<_>>();
        if coverage.is_empty() || !seen_routes.insert((center_delta_micro.clone(), threshold_micro))
        {
            continue;
        }
        let maximum_positive = coverage
            .iter()
            .map(|support_index| support_margins[*support_index])
            .max()
            .unwrap_or(i64::MIN);
        let frame_id = support[index].frame_id_sha256.clone();
        candidates.push(MedoidCandidate {
            coverage,
            gap_micro: maximum_positive.saturating_sub(maximum_negative),
            threshold_micro,
            frame_id,
            center_delta_micro,
        });
    }

    let mut uncovered = (0..support.len()).collect::<BTreeSet<_>>();
    let mut selected = Vec::<MedoidCandidate>::new();
    let mut selected_indices = BTreeSet::<usize>::new();
    while !uncovered.is_empty() && selected.len() < 8 {
        let mut best = None::<(usize, usize)>;
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            if selected_indices.contains(&candidate_index) {
                continue;
            }
            let newly_covered = candidate.coverage.intersection(&uncovered).count();
            if newly_covered == 0 {
                continue;
            }
            let replace = best.is_none_or(|(best_index, best_newly_covered)| {
                let current = &candidates[best_index];
                newly_covered > best_newly_covered
                    || (newly_covered == best_newly_covered
                        && (candidate.coverage.len() > current.coverage.len()
                            || (candidate.coverage.len() == current.coverage.len()
                                && (candidate.gap_micro > current.gap_micro
                                    || (candidate.gap_micro == current.gap_micro
                                        && (candidate.threshold_micro
                                            < current.threshold_micro
                                            || (candidate.threshold_micro
                                                == current.threshold_micro
                                                && candidate.frame_id < current.frame_id)))))))
            });
            if replace {
                best = Some((candidate_index, newly_covered));
            }
        }
        let Some((best_index, _)) = best else {
            break;
        };
        let candidate = candidates[best_index].clone();
        for covered in &candidate.coverage {
            uncovered.remove(covered);
        }
        selected_indices.insert(best_index);
        selected.push(candidate);
    }
    if support.len().saturating_sub(uncovered.len()) < 32 {
        return None;
    }

    while selected.len() < 8 && selected_indices.len() < candidates.len() {
        let mut best = None::<(usize, u64)>;
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            if selected_indices.contains(&candidate_index) {
                continue;
            }
            let minimum_center_distance = selected
                .iter()
                .map(|current| {
                    candidate
                        .center_delta_micro
                        .iter()
                        .zip(&current.center_delta_micro)
                        .fold(0_u64, |distance, (left, right)| {
                            distance.saturating_add(u64::from(left.abs_diff(*right)))
                        })
                })
                .min()
                .unwrap_or(u64::MAX);
            let replace = best.is_none_or(|(best_index, best_distance)| {
                let current = &candidates[best_index];
                minimum_center_distance > best_distance
                    || (minimum_center_distance == best_distance
                        && (candidate.coverage.len() > current.coverage.len()
                            || (candidate.coverage.len() == current.coverage.len()
                                && (candidate.gap_micro > current.gap_micro
                                    || (candidate.gap_micro == current.gap_micro
                                        && (candidate.threshold_micro
                                            < current.threshold_micro
                                            || (candidate.threshold_micro
                                                == current.threshold_micro
                                                && candidate.frame_id < current.frame_id)))))))
            });
            if replace {
                best = Some((candidate_index, minimum_center_distance));
            }
        }
        let Some((best_index, _)) = best else {
            break;
        };
        selected_indices.insert(best_index);
        selected.push(candidates[best_index].clone());
    }

    let primary = selected.first()?.clone();
    let subcenters = selected
        .into_iter()
        .skip(1)
        .map(|candidate| LearnedWaveSubcenter {
            center_delta_micro: candidate.center_delta_micro,
            threshold_micro: candidate.threshold_micro,
        })
        .collect();
    Some(LearnedWaveRoute {
        cells: cells as u16,
        center_delta_micro: primary.center_delta_micro,
        threshold_micro: primary.threshold_micro,
        query_atom_ids,
        subcenters,
    })
}

pub(crate) fn learned_wave_route_accepts_frame(
    route: &LearnedWaveRoute,
    frame: &crate::RelationFrame,
) -> bool {
    let cells = usize::from(route.cells);
    if cells == 0 || route.center_delta_micro.len() != cells.saturating_mul(2) {
        return false;
    }
    let mut atoms = crate::relation_frame_online_routing_atom_ids(frame);
    if !route.query_atom_ids.is_empty() {
        atoms.retain(|atom| route.query_atom_ids.binary_search(atom).is_ok());
    }
    if atoms.is_empty() {
        return false;
    }
    let query = phase_vector_from_atom_ids(atoms, cells);
    let score = |center_delta_micro: &[i32]| {
        if center_delta_micro.len() != cells.saturating_mul(2) {
            return i64::MIN;
        }
        phase_margin_to_micro(
            query
                .iter()
                .zip(center_delta_micro.chunks_exact(2))
                .map(|(cell, center)| {
                    cell.re * f64::from(center[0]) / 1_000_000.0
                        + cell.im * f64::from(center[1]) / 1_000_000.0
                })
                .sum::<f64>()
                / cells as f64,
        )
        .unwrap_or(i64::MIN)
    };
    score(&route.center_delta_micro) >= route.threshold_micro
        || route
            .subcenters
            .iter()
            .any(|subcenter| score(&subcenter.center_delta_micro) >= subcenter.threshold_micro)
}

fn learned_wave_feature_vocabulary(
    support: &[crate::RelationFrame],
    negatives: &[crate::RelationFrame],
    limit: usize,
) -> Vec<u64> {
    let mut counts = BTreeMap::<u64, (usize, usize)>::new();
    for frame in support {
        for atom in crate::relation_frame_online_routing_atom_ids(frame) {
            counts.entry(atom).or_default().0 += 1;
        }
    }
    for frame in negatives {
        for atom in crate::relation_frame_online_routing_atom_ids(frame) {
            counts.entry(atom).or_default().1 += 1;
        }
    }
    let support_rows = support.len().max(1);
    let negative_rows = negatives.len().max(1);
    let mut ranked = counts
        .into_iter()
        .filter(|(_, (positive, negative))| *positive >= 2 || *negative >= 2)
        .map(|(atom, (positive, negative))| {
            let separation = positive
                .saturating_mul(negative_rows)
                .abs_diff(negative.saturating_mul(support_rows));
            (atom, separation, positive.saturating_add(negative))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.0.cmp(&right.0))
    });
    ranked.truncate(limit);
    let mut atoms = ranked
        .into_iter()
        .map(|(atom, _, _)| atom)
        .collect::<Vec<_>>();
    atoms.sort_unstable();
    atoms
}

pub(crate) fn phase_clean_support_future(
    package: &mut ResponsePackage,
    support: &[crate::RelationFrame],
    future: &[crate::RelationFrame],
    _negatives: &[crate::RelationFrame],
) -> (Vec<crate::RelationFrame>, Vec<crate::RelationFrame>) {
    let threshold = package.wave_margin_micro;
    let clean_support = support
        .iter()
        .filter(|frame| {
            crate::relation_frame_phase_margin_micro(package, frame)
                .is_some_and(|margin| margin >= threshold)
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut clean_future = future
        .iter()
        .filter(|frame| {
            crate::relation_frame_phase_margin_micro(package, frame)
                .is_some_and(|margin| margin >= threshold)
        })
        .cloned()
        .collect::<Vec<_>>();
    clean_future.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });
    clean_future.dedup_by(|left, right| left.frame_id_sha256 == right.frame_id_sha256);
    (clean_support, clean_future)
}

#[cfg(test)]
fn select_diverse_future(
    future: &[crate::RelationFrame],
    limit: usize,
) -> Vec<crate::RelationFrame> {
    let mut selected_ids = BTreeSet::new();
    let mut sessions = BTreeSet::new();
    let mut surfaces = BTreeSet::new();
    let mut selected = Vec::with_capacity(limit.min(future.len()));
    for frame in future {
        if sessions.insert(frame.session_id_sha256.as_str())
            && selected_ids.insert(frame.frame_id_sha256.as_str())
        {
            selected.push(frame.clone());
            if selected.len() == limit {
                return selected;
            }
        }
    }
    for frame in future {
        let Some(surface) = relation_frame_structural_family_id(frame) else {
            continue;
        };
        if surfaces.insert(surface) && selected_ids.insert(frame.frame_id_sha256.as_str()) {
            selected.push(frame.clone());
            if selected.len() == limit {
                return selected;
            }
        }
    }
    for frame in future {
        if selected_ids.insert(frame.frame_id_sha256.as_str()) {
            selected.push(frame.clone());
            if selected.len() == limit {
                break;
            }
        }
    }
    selected.sort_by(|left, right| {
        left.observed_at_unix_nanos
            .cmp(&right.observed_at_unix_nanos)
            .then_with(|| left.frame_id_sha256.cmp(&right.frame_id_sha256))
    });
    selected
}

pub fn build_online_collection_admission_snapshot(
    candidates: &[OnlineCollectionAdmissionCandidate],
    project_id: &str,
    revision: u64,
    now_unix: u64,
    max_age_seconds: u64,
    gate_build_sha256: &str,
    runtime_build_sha256: &str,
) -> Result<Option<OnlineAdmissionSnapshot>, &'static str> {
    let mut packages = Vec::new();
    let mut receipt_digests = BTreeMap::new();
    for candidate in candidates {
        let package = &candidate.package;
        if candidate.causal_report.verdict != "PASS"
            || candidate.causal_report.package_id != package.package_id
            || online_collection_support_manifest_digest(candidate)
                .map_err(|_| "online_collection_admission_support_manifest_encode_failed")?
                != candidate.support_manifest_sha256
            || online_collection_future_manifest_digest(candidate)
                .map_err(|_| "online_collection_admission_future_manifest_encode_failed")?
                != candidate.future_manifest_sha256
            || candidate.support_receipts.len() < 32
            || candidate.future_receipts.len() < 32
            || candidate
                .support_receipts
                .iter()
                .any(|receipt| !receipt.verifier_pass)
            || candidate
                .future_receipts
                .iter()
                .any(|receipt| !receipt.verifier_pass)
            || !package.eligible_for_admission_candidate()
        {
            continue;
        }
        let support_intents = candidate
            .support_receipts
            .iter()
            .map(|receipt| receipt.client_intent_id_sha256.as_str())
            .collect::<BTreeSet<_>>();
        if candidate.future_receipts.iter().any(|receipt| {
            support_intents.contains(receipt.client_intent_id_sha256.as_str())
                || receipt.event_time_unix_nanos.is_none_or(|event_time| {
                    event_time <= candidate.support_watermark_event_time_unix_nanos
                })
        }) {
            continue;
        }
        let future_refs = candidate
            .future_receipts
            .iter()
            .map(|receipt| receipt.evidence_graph_sha256.as_str())
            .collect::<BTreeSet<_>>();
        let parity_sha256 =
            execute_runtime_parity(package, &candidate.runtime_parity_cases, &future_refs)?.or(
                validate_durable_runtime_parity(
                    package,
                    &candidate.durable_runtime_parity_receipts,
                    &future_refs,
                )?,
            );
        let Some(parity_sha256) = parity_sha256 else {
            continue;
        };
        let future_sha256 = canonical_json_sha256(&FrameReceiptSet {
            schema: RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2,
            package_id: &package.package_id,
            frame_ids: candidate
                .future_receipts
                .iter()
                .map(|receipt| receipt.evidence_graph_sha256.as_str())
                .collect(),
            wrong_accepts: 0,
        })?;
        receipt_digests.insert(
            package.package_id.clone(),
            (
                candidate.support_manifest_sha256.clone(),
                canonical_json_sha256(&candidate.causal_report)?,
                parity_sha256,
                future_sha256,
                canonical_json_sha256(&(
                    RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1,
                    "exact_singleton",
                    package.package_id.as_str(),
                    &package.program,
                    &candidate.causal_report,
                ))?,
            ),
        );
        packages.push(package.clone());
    }
    if packages.is_empty() {
        return Ok(None);
    }
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    packages.dedup_by(|left, right| left.package_id == right.package_id);
    let registry = ResponseRegistry {
        schema: RESPONSE_REGISTRY_SCHEMA_V6.to_owned(),
        revision,
        packages,
    };
    registry.validate()?;
    let registry_sha256 = response_registry_digest(&registry)?;
    let mut bindings = Vec::new();
    for package in &registry.packages {
        let (support, causal, parity, future, semantic_alias) = receipt_digests
            .remove(&package.package_id)
            .ok_or("online_collection_admission_receipts_missing")?;
        let verifier = package
            .verifier
            .as_ref()
            .ok_or("online_collection_admission_verifier_missing")?;
        let mut binding = ResponsePackageAuthorityBindingV2 {
            package_id: package.package_id.clone(),
            registry_revision: revision,
            package_sha256: response_package_digest(package)?,
            execution_payload_sha256: response_execution_payload_digest(package)?,
            actor_program_sha256: response_actor_program_digest(&package.program)?,
            independent_verifier_program_sha256: response_independent_verifier_program_digest(
                verifier,
            )?,
            verifier_schema: package.proof.verifier_schema.clone(),
            support_manifest_schema: RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1.to_owned(),
            support_manifest_sha256: support,
            exact_causal_proof_schema: RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2.to_owned(),
            exact_causal_proof_sha256: causal,
            runtime_parity_receipt_set_schema: RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1
                .to_owned(),
            runtime_parity_receipt_set_sha256: parity,
            future_verifier_receipt_set_schema: RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2
                .to_owned(),
            future_verifier_receipt_set_sha256: future,
            semantic_alias_proof_schema: RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1.to_owned(),
            semantic_alias_proof_sha256: semantic_alias,
            proof_receipts_sha256: String::new(),
        };
        binding.proof_receipts_sha256 = response_proof_receipts_digest(&binding)?;
        bindings.push(binding);
    }
    let admission = CompositeResponseAdmissionV2 {
        schema: COMPOSITE_ADMISSION_SCHEMA_V2.to_owned(),
        project_id: project_id.to_owned(),
        generated_at_unix: now_unix,
        expires_at_unix: now_unix.saturating_add(max_age_seconds),
        verdict: "PASS".to_owned(),
        eligible_for_local_accept: true,
        response_authority: ResponseAuthorityV2 {
            schema: RESPONSE_AUTHORITY_SCHEMA_V2.to_owned(),
            registry_schema: registry.schema.clone(),
            registry_revision: revision,
            registry_sha256,
            gate_build_sha256: gate_build_sha256.to_owned(),
            runtime_build_sha256: runtime_build_sha256.to_owned(),
            packages: bindings,
        },
    };
    Ok(Some(OnlineAdmissionSnapshot {
        registry,
        admission,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AtomSource, AtomValueType, OnlineResponseCandidate, RELATION_FRAME_SCHEMA, RelationAtom,
        RelationFrame, ResponseValueSelector, SOURCE_NEUTRAL_EXTRACTOR_VERSION,
        synthesize_response_operator,
    };
    use nando_core::wave::{PhaseCenterCell, PhaseCenterFlatRecord, phase_vector_from_atom_ids};

    fn frame(index: usize) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: format!("{index:064x}"),
            event_id_sha256: format!("{:064x}", index + 1),
            client_intent_id_sha256: "c".repeat(64),
            session_id_sha256: format!("{:064x}", index % 4),
            observed_at_unix_nanos: index as u64,
            estimated_input_tokens: 100,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::ToolKind {
                    value: "exec".to_owned(),
                },
                RelationAtom::ObservationCallShape {
                    value: format!("surface-{}", index % 2),
                },
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::ResponseShape {
                    value: "function_call".to_owned(),
                },
                RelationAtom::ClientCapabilityAtom {
                    atom_id: crate::package::stable_atom_id(
                        "client_capability:function:write_stdin",
                    ),
                },
                RelationAtom::TypedSlot {
                    slot_id: 1,
                    value_type: AtomValueType::Identifier,
                    source: AtomSource::Observation,
                    value_sha256: "a".repeat(64),
                },
                RelationAtom::UniqueSlot { slot_id: 1 },
                RelationAtom::ObservationSelector {
                    slot_id: 1,
                    selector: ResponseValueSelector::UniqueScalar {
                        value_type: AtomValueType::Identifier,
                    },
                },
                RelationAtom::TypedSlot {
                    slot_id: 2,
                    value_type: AtomValueType::Identifier,
                    source: AtomSource::Action,
                    value_sha256: "a".repeat(64),
                },
                RelationAtom::SlotEquality {
                    left_slot: 1,
                    right_slot: 2,
                },
                RelationAtom::ActionFunction {
                    value: "write_stdin".to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "session_id".to_owned(),
                    slot_id: 2,
                    value_type: None,
                },
            ],
            evidence_ref_sha256: format!("{:064x}", index + 10_000),
        }
    }

    fn candidate(
        support: Vec<RelationFrame>,
        future: Vec<RelationFrame>,
    ) -> OnlineResponseAdmissionCandidate {
        let synthesized = synthesize_response_operator(&support).expect("synthesis");
        let program = synthesized.candidate.program;
        let runtime_parity_cases = future
            .iter()
            .enumerate()
            .map(|(index, frame)| {
                let provider_payload = serde_json::json!({
                    "input": [{
                        "type": "function_call_output",
                        "output": serde_json::to_string(&format!("session-{index}"))
                            .expect("scalar")
                    }]
                });
                let expected_response = crate::execute_response(&program, "", &provider_payload)
                    .response
                    .expect("runtime fixture executes");
                crate::RuntimeParityCase {
                    evidence_ref_sha256: frame.frame_id_sha256.clone(),
                    request_text: String::new(),
                    provider_payload,
                    expected_response,
                }
            })
            .collect();
        let mut positive_center = vec![PhaseCenterCell::default(); 32];
        for frame in &support {
            for (sum, cell) in positive_center.iter_mut().zip(phase_vector_from_atom_ids(
                crate::relation_frame_online_routing_atom_ids(frame),
                32,
            )) {
                sum.re += cell.re / support.len() as f64;
                sum.im += cell.im / support.len() as f64;
            }
        }
        let wave_runtime_package = PhaseCenterFlatRuntime::new(
            32,
            vec![PhaseCenterFlatRecord {
                positive_center: positive_center.into_boxed_slice(),
                negative_center: vec![PhaseCenterCell::default(); 32].into_boxed_slice(),
            }],
        )
        .expect("wave runtime")
        .to_bytes()
        .expect("wave package");
        OnlineResponseAdmissionCandidate {
            candidate: OnlineResponseCandidate {
                bucket_id: 1,
                structural_family_id: 1,
                teacher_signature_sha256: "d".repeat(64),
                positive_rows: support.len() + future.len(),
                negative_rows: 0,
                positive_tokens: 0,
                negative_tokens: 0,
                distinct_sessions: 4,
                wave_threshold_micro: 1,
                wave_runtime_bytes: 1,
                wave_runtime_fingerprint64: 1,
                program,
                verifier: synthesized.verifier,
                phase_rank: synthesized.candidate.phase_rank,
                exact_checks: synthesized.candidate.exact_checks,
            },
            wave_runtime_package,
            support,
            future,
            negatives: Vec::new(),
            required_routing_atom_ids: Vec::new(),
            runtime_parity_cases,
            semantic_alias_edges: Vec::new(),
        }
    }

    #[test]
    fn online_admission_rejects_support_future_overlap() {
        let support = (0..32).map(frame).collect::<Vec<_>>();
        let candidate = candidate(support.clone(), support);
        let snapshot = build_online_admission_snapshot(
            &[candidate],
            "project",
            1,
            100,
            60,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("admission evaluation");
        assert!(snapshot.is_none());
    }

    #[test]
    fn merged_authority_revision_depends_on_active_content_not_candidate_revision() {
        let support = (0..32).map(frame).collect::<Vec<_>>();
        let future = (32..64)
            .map(|index| {
                let mut row = frame(index);
                row.session_id_sha256 = format!("{:064x}", index + 10_000);
                row
            })
            .collect::<Vec<_>>();
        let build = |revision| {
            let snapshot = build_online_admission_snapshot(
                &[candidate(support.clone(), future.clone())],
                "project",
                revision,
                100,
                60,
                &"a".repeat(64),
                &"b".repeat(64),
            )
            .expect("admission evaluation")
            .expect("complete candidate");
            merge_online_admission_snapshots(vec![snapshot])
                .expect("merge")
                .expect("merged authority")
        };
        let first = build(7);
        let second = build(99);
        assert_eq!(first.registry.revision, second.registry.revision);
        assert_eq!(
            response_registry_digest(&first.registry),
            response_registry_digest(&second.registry)
        );
        assert_eq!(
            first.admission.response_authority.packages,
            second.admission.response_authority.packages
        );
    }

    #[test]
    fn semantic_law_binding_preserves_consensus_actor_and_independent_verifier() {
        let first = (0..32).map(frame).collect::<Vec<_>>();
        let mut second = (100..132).map(frame).collect::<Vec<_>>();
        for row in &mut second {
            for atom in &mut row.atoms {
                match atom {
                    RelationAtom::ActionFunction { value } => *value = "wait".to_owned(),
                    RelationAtom::ActionRoleArgument { name, .. } => {
                        *name = "cell_id".to_owned();
                    }
                    _ => {}
                }
            }
        }
        let first_program = synthesize_response_operator(&first)
            .expect("first adapter")
            .candidate
            .program;
        let second_program = synthesize_response_operator(&second)
            .expect("second adapter")
            .candidate
            .program;
        let consensus = crate::ResponseProgram::unique_consensus(vec![
            crate::ResponseConsensusVariant {
                program: first_program,
                allowed_layout_sha256: Vec::new(),
                required_request_atom_ids: Vec::new(),
            },
            crate::ResponseConsensusVariant {
                program: second_program,
                allowed_layout_sha256: Vec::new(),
                required_request_atom_ids: Vec::new(),
            },
        ]);
        let mut package = compile_source_neutral_quarantine_packages(&first, true)
            .into_iter()
            .next()
            .expect("package shell");
        let training = first.into_iter().chain(second).collect::<Vec<_>>();

        bind_proven_semantic_law_program(&mut package, &consensus, &training)
            .expect("bind semantic law");

        assert_eq!(package.program, consensus);
        assert!(matches!(
            package.verifier,
            Some(crate::VerifierProgram::UniqueConsensus { .. })
        ));
        assert!(package.validate().is_ok());
    }

    #[test]
    fn runtime_parity_normalizes_only_execution_budgets() {
        let actual = r#"{"name":"wait","arguments":{"cell_id":"abc","yield_time_ms":10000}}"#;
        let teacher = r#"{"name":"wait","arguments":{"cell_id":"abc","yield_time_ms":30000}}"#;
        let wrong_handle = r#"{"name":"wait","arguments":{"cell_id":"xyz","yield_time_ms":30000}}"#;
        let destructive = r#"{"name":"wait","arguments":{"cell_id":"abc","yield_time_ms":30000,"terminate":true}}"#;

        assert!(responses_match_after_execution_budget_normalization(
            actual, teacher
        ));
        assert!(!responses_match_after_execution_budget_normalization(
            actual,
            wrong_handle
        ));
        assert!(!responses_match_after_execution_budget_normalization(
            actual,
            destructive
        ));
    }

    #[test]
    fn durable_parity_rejects_non_exact_teacher_match_with_valid_digest() {
        let program_sha256 = "a".repeat(64);
        let verifier_sha256 = "b".repeat(64);
        let mut receipt = DurableRuntimeParityReceipt {
            schema: DURABLE_RUNTIME_PARITY_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_sha256: String::new(),
            evidence_ref_sha256: "c".repeat(64),
            program_sha256: program_sha256.clone(),
            verifier_sha256: verifier_sha256.clone(),
            input_sha256: "d".repeat(64),
            teacher_response_sha256: "e".repeat(64),
            actor_response_sha256: "f".repeat(64),
            actor_executed: true,
            teacher_authority_match: true,
            independent_verifier_pass: true,
            exact_teacher_match: false,
        };
        receipt.receipt_sha256 =
            durable_runtime_parity_receipt_digest(&receipt).expect("receipt digest");

        assert!(!validate_durable_runtime_parity_receipt(
            &receipt,
            &program_sha256,
            &verifier_sha256
        ));
    }

    #[test]
    fn support_parity_restores_typed_role_and_empty_poll_argument() {
        let mut support = frame(1);
        support
            .atoms
            .retain(|atom| !matches!(atom, RelationAtom::ObservationSelector { .. }));
        support.atoms.push(RelationAtom::ObservationSelector {
            slot_id: 1,
            selector: ResponseValueSelector::ContentLinePrefix {
                prefix: "Process running with session ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
        });
        support.atoms.sort();
        let expected_response = serde_json::json!({
            "name": "write_stdin",
            "arguments": {
                "session_id": 42,
                "chars": "",
                "yield_time_ms": 1000,
                "max_output_tokens": 8000
            }
        })
        .to_string();
        let parity = crate::RuntimeParityCase {
            evidence_ref_sha256: support.frame_id_sha256.clone(),
            request_text: String::new(),
            provider_payload: serde_json::json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "Process running with session ID 42"
                }]
            }),
            expected_response: expected_response.clone(),
        };
        let enriched = action_schema_enriched_frame(&support, Some(&parity));
        let synthesized = synthesize_response_operator(&[enriched]).expect("typed synthesis");
        let execution =
            crate::execute_response(&synthesized.candidate.program, "", &parity.provider_payload);
        assert_eq!(
            execution.response.as_deref(),
            Some(expected_response.as_str())
        );
        crate::verify_response_independently(
            &synthesized.verifier,
            &parity.provider_payload,
            &expected_response,
        )
        .expect("independent typed verifier");
    }

    #[test]
    fn online_admission_builds_authorized_executor_for_disjoint_proof() {
        let support = (0..32).map(frame).collect::<Vec<_>>();
        let future = (32..64).map(frame).collect::<Vec<_>>();
        synthesize_response_operator(&support).expect("direct synthesis");
        let packages = compile_source_neutral_quarantine_packages(&support, true);
        assert!(!packages.is_empty(), "package synthesis failed");
        let package = packages.into_iter().next().expect("package");
        let causal = evaluate_grounded_wave_causality(&package, &support, &future, &[]);
        let snapshot = build_online_admission_snapshot(
            &[candidate(support, future)],
            "project",
            64,
            100,
            60,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("admission evaluation")
        .unwrap_or_else(|| panic!("proven admission: {causal:?}"));
        let executor = crate::ResponseExecutor::from_registry_with_admission(
            snapshot.registry,
            snapshot.admission,
            "project",
            &"a".repeat(64),
            &"b".repeat(64),
            100,
            60,
        )
        .expect("authorized executor");
        assert_eq!(executor.active_package_count(), 1);
    }

    #[test]
    fn online_admission_uses_clean_subcenter_atoms_as_wave_query() {
        let guard_atom = crate::package::stable_atom_id("test:clean-subcenter");
        let support = (0..32)
            .map(|index| {
                let mut frame = frame(index);
                frame.atoms.push(RelationAtom::ClientCapabilityAtom {
                    atom_id: guard_atom,
                });
                frame
            })
            .collect::<Vec<_>>();
        let future = (32..64)
            .map(|index| {
                let mut frame = frame(index);
                frame.atoms.push(RelationAtom::ClientCapabilityAtom {
                    atom_id: guard_atom,
                });
                frame
            })
            .collect::<Vec<_>>();
        let mut candidate = candidate(support, future);
        let center = phase_vector_from_atom_ids([guard_atom], 32);
        candidate.wave_runtime_package = PhaseCenterFlatRuntime::new(
            32,
            vec![PhaseCenterFlatRecord {
                positive_center: center.into_boxed_slice(),
                negative_center: vec![PhaseCenterCell::default(); 32].into_boxed_slice(),
            }],
        )
        .expect("wave runtime")
        .to_bytes()
        .expect("wave package");
        candidate.required_routing_atom_ids = vec![guard_atom];
        candidate.candidate.wave_threshold_micro = 900_000;

        let snapshot = build_online_admission_snapshot(
            &[candidate],
            "project",
            64,
            100,
            60,
            &"a".repeat(64),
            &"b".repeat(64),
        )
        .expect("admission evaluation");
        assert!(snapshot.is_some());
    }

    #[test]
    fn learned_wave_route_uses_two_clean_centers_for_two_positive_clusters() {
        let cluster_frame = |index: usize, atom_id: u64| {
            let mut frame = frame(index);
            frame.atoms = vec![RelationAtom::RequestPhaseAtom { atom_id }];
            frame
        };
        let route_accepts = |route: &LearnedWaveRoute, frame: &RelationFrame| {
            let mut atoms = crate::relation_frame_online_routing_atom_ids(frame);
            if !route.query_atom_ids.is_empty() {
                atoms.retain(|atom| route.query_atom_ids.binary_search(atom).is_ok());
            }
            let query = phase_vector_from_atom_ids(atoms, usize::from(route.cells));
            let score = |center_delta_micro: &[i32]| {
                phase_margin_to_micro(
                    query
                        .iter()
                        .zip(center_delta_micro.chunks_exact(2))
                        .map(|(cell, delta)| {
                            cell.re * f64::from(delta[0]) / 1_000_000.0
                                + cell.im * f64::from(delta[1]) / 1_000_000.0
                        })
                        .sum::<f64>()
                        / f64::from(route.cells),
                )
                .expect("finite route score")
            };
            score(&route.center_delta_micro) >= route.threshold_micro
                || route.subcenters.iter().any(|subcenter| {
                    score(&subcenter.center_delta_micro) >= subcenter.threshold_micro
                })
        };

        let negatives = (0..16)
            .map(|index| cluster_frame(10_000 + index, 10_000))
            .collect::<Vec<_>>();
        let (support, route) = (2..256)
            .find_map(|second_atom| {
                let support = (0..32)
                    .map(|index| cluster_frame(index, if index < 16 { 1 } else { second_atom }))
                    .collect::<Vec<_>>();
                let route = learned_wave_route_from_support_medoid(&support, &negatives, 32)?;
                let primary_only = LearnedWaveRoute {
                    subcenters: Vec::new(),
                    ..route.clone()
                };
                (!route.subcenters.is_empty()
                    && support
                        .iter()
                        .any(|frame| !route_accepts(&primary_only, frame)))
                .then_some((support, route))
            })
            .expect("two separable positive clusters");

        assert_eq!(route.subcenters.len(), 1);
        assert!(support.iter().all(|frame| route_accepts(&route, frame)));
        assert!(negatives.iter().all(|frame| !route_accepts(&route, frame)));
        let primary_only = LearnedWaveRoute {
            subcenters: Vec::new(),
            ..route.clone()
        };
        assert!(
            support
                .iter()
                .any(|frame| !route_accepts(&primary_only, frame))
        );
    }

    #[test]
    fn learned_wave_route_keeps_clean_support_and_abstains_on_collisions() {
        let cluster_frame = |index: usize, atom_id: u64| {
            let mut frame = frame(index);
            frame.atoms = vec![RelationAtom::RequestPhaseAtom { atom_id }];
            frame
        };
        let route_accepts = |route: &LearnedWaveRoute, frame: &RelationFrame| {
            let mut atoms = crate::relation_frame_online_routing_atom_ids(frame);
            atoms.retain(|atom| route.query_atom_ids.binary_search(atom).is_ok());
            let query = phase_vector_from_atom_ids(atoms, usize::from(route.cells));
            let score = |center_delta_micro: &[i32]| {
                phase_margin_to_micro(
                    query
                        .iter()
                        .zip(center_delta_micro.chunks_exact(2))
                        .map(|(cell, delta)| {
                            cell.re * f64::from(delta[0]) / 1_000_000.0
                                + cell.im * f64::from(delta[1]) / 1_000_000.0
                        })
                        .sum::<f64>()
                        / f64::from(route.cells),
                )
                .expect("finite route score")
            };
            score(&route.center_delta_micro) >= route.threshold_micro
                || route.subcenters.iter().any(|subcenter| {
                    score(&subcenter.center_delta_micro) >= subcenter.threshold_micro
                })
        };

        let support = (0..40)
            .map(|index| cluster_frame(index, if index < 32 { 1 } else { 10_000 }))
            .collect::<Vec<_>>();
        let negatives = (0..16)
            .map(|index| cluster_frame(10_000 + index, 10_000))
            .collect::<Vec<_>>();
        let route = learned_wave_route_from_support_medoid(&support, &negatives, 32)
            .expect("clean support subcenter");

        assert_eq!(
            support
                .iter()
                .filter(|frame| route_accepts(&route, frame))
                .count(),
            32
        );
        assert!(negatives.iter().all(|frame| !route_accepts(&route, frame)));
    }

    #[test]
    fn process_session_protocol_has_structural_capability_but_cell_wait_does_not() {
        let process = crate::ResponseProgram::function_call_from_roles(
            "write_stdin",
            ResponseValueSelector::ContentLinePrefix {
                prefix: "Process running with session ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            vec![],
        );
        let cell = crate::ResponseProgram::function_call_from_roles(
            "wait",
            ResponseValueSelector::ContentLinePrefix {
                prefix: "Script running with cell ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            vec![],
        );
        assert!(program_required_client_capability_atom(&process).is_none());
        assert!(program_required_client_capability_atom(&cell).is_some());
    }

    #[test]
    fn future_selection_preserves_session_diversity_before_filling() {
        let future = (0..80)
            .map(|index| {
                let mut frame = frame(index + 100);
                frame.session_id_sha256 = if index < 40 {
                    "1".repeat(64)
                } else if index < 70 {
                    "2".repeat(64)
                } else {
                    "3".repeat(64)
                };
                frame
            })
            .collect::<Vec<_>>();
        let selected = select_diverse_future(&future, 32);
        assert_eq!(selected.len(), 32);
        assert_eq!(
            selected
                .iter()
                .map(|frame| frame.session_id_sha256.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            3
        );
    }
}
