use std::collections::BTreeMap;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    ResponsePackage, ResponseProgram, ResponseRegistry, ResponseRoutingPredicate, VerifierProgram,
    response_program_external_verifier_schema,
};

pub use nando_operator_admission::{
    COMPOSITE_ADMISSION_SCHEMA_V2, CompositeResponseAdmissionV2, FinalizedPostVerifierReceiptV1,
    FinalizedRuntimeVerificationReceiptV2, PostVerifierReceiptV1, RESPONSE_AUTHORITY_SCHEMA_V2,
    RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2, RESPONSE_FUTURE_VERIFIER_RECEIPT_SCHEMA_V2,
    RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2, RESPONSE_POST_VERIFIER_ADMISSION_SCHEMA_V1,
    RESPONSE_POST_VERIFIER_RECEIPT_SCHEMA_V1, RESPONSE_PROOF_RECEIPT_BINDING_SCHEMA_V1,
    RESPONSE_REGISTRY_SCHEMA_V6, RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1,
    RESPONSE_RUNTIME_RECEIPT_SCHEMA_V2, RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1,
    RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1, ResponseAuthorityV2, ResponsePackageAuthorityBindingV2,
    RuntimeVerificationReceiptV2, RuntimeVerificationResultV2, finalize_post_verifier_receipt,
    response_proof_receipts_digest,
};
pub use nando_operator_kernel::canonical::{
    canonical_json_bytes, canonical_json_sha256, sha256_bytes, valid_nonzero_sha256,
};

pub(crate) use nando_operator_admission::{
    IndependentlyVerifiedExecution, ValidatedResponseAuthority, finalize_runtime_receipt,
};

pub const RESPONSE_EXECUTION_PAYLOAD_SCHEMA_V1: &str = "nando.response-execution-payload.v1";
pub const RESPONSE_RUNTIME_CONTRACT_SCHEMA_V1: &str = "nando.response-runtime-contract.v1";

#[derive(Serialize)]
struct ResponseExecutionPayloadDigestMaterial<'a> {
    schema: &'static str,
    package_schema: &'a str,
    package_id: &'a str,
    origin: crate::ResponsePackageOrigin,
    program: &'a ResponseProgram,
    verifier: &'a Option<VerifierProgram>,
    routing_predicates: &'a [ResponseRoutingPredicate],
    required_routing_atom_ids: &'a [u64],
    phase_centers: &'a [u64],
    anti_centers: &'a [u64],
    wave_margin_micro: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    crystallized_operator: &'a Option<crate::VerifiedOperatorRestartBundle>,
}

#[derive(Serialize)]
struct RegistryPackageDigestMaterial<'a> {
    package_id: &'a str,
    package_sha256: String,
}

#[derive(Serialize)]
struct ResponseRegistryDigestMaterial<'a> {
    schema: &'a str,
    revision: u64,
    packages: Vec<RegistryPackageDigestMaterial<'a>>,
}

#[must_use]
pub fn response_runtime_contract_sha256() -> String {
    let sources: [&[u8]; 7] = [
        include_bytes!("authority.rs"),
        include_bytes!("package.rs"),
        include_bytes!("program.rs"),
        include_bytes!("runtime.rs"),
        include_bytes!("verifier.rs"),
        include_bytes!("grounding.rs"),
        include_bytes!("contracts.rs"),
    ];
    let mut digest = Sha256::new();
    digest.update(RESPONSE_RUNTIME_CONTRACT_SCHEMA_V1.as_bytes());
    for source in sources {
        digest.update(
            u64::try_from(source.len())
                .unwrap_or(u64::MAX)
                .to_le_bytes(),
        );
        digest.update(source);
    }
    format!("{:x}", digest.finalize())
}

pub fn response_execution_payload_digest(
    package: &ResponsePackage,
) -> Result<String, &'static str> {
    canonical_json_sha256(&ResponseExecutionPayloadDigestMaterial {
        schema: RESPONSE_EXECUTION_PAYLOAD_SCHEMA_V1,
        package_schema: &package.schema,
        package_id: &package.package_id,
        origin: package.origin,
        program: &package.program,
        verifier: &package.verifier,
        routing_predicates: &package.routing_predicates,
        required_routing_atom_ids: &package.required_routing_atom_ids,
        phase_centers: &package.phase_centers,
        anti_centers: &package.anti_centers,
        wave_margin_micro: package.wave_margin_micro,
        crystallized_operator: &package.crystallized_operator,
    })
}

pub fn response_package_digest(package: &ResponsePackage) -> Result<String, &'static str> {
    canonical_json_sha256(package)
}

pub fn response_actor_program_digest(program: &ResponseProgram) -> Result<String, &'static str> {
    canonical_json_sha256(program)
}

pub fn response_independent_verifier_program_digest(
    verifier: &VerifierProgram,
) -> Result<String, &'static str> {
    canonical_json_sha256(verifier)
}

pub fn response_registry_digest(registry: &ResponseRegistry) -> Result<String, &'static str> {
    let mut packages = registry.packages.iter().collect::<Vec<_>>();
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    if packages
        .windows(2)
        .any(|pair| pair[0].package_id == pair[1].package_id)
    {
        return Err("duplicate_package_id");
    }
    let packages = packages
        .into_iter()
        .map(|package| {
            Ok(RegistryPackageDigestMaterial {
                package_id: &package.package_id,
                package_sha256: response_package_digest(package)?,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    canonical_json_sha256(&ResponseRegistryDigestMaterial {
        schema: &registry.schema,
        revision: registry.revision,
        packages,
    })
}

pub(crate) type AdmissionReceiptDigests = (String, String, String, String, String);

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_composite_admission_for_registry(
    registry: &ResponseRegistry,
    mut receipt_digests: BTreeMap<String, AdmissionReceiptDigests>,
    project_id: &str,
    now_unix: u64,
    max_age_seconds: u64,
    gate_build_sha256: &str,
    runtime_build_sha256: &str,
    missing_receipts_error: &'static str,
    missing_verifier_error: &'static str,
) -> Result<CompositeResponseAdmissionV2, &'static str> {
    registry.validate()?;
    let registry_sha256 = response_registry_digest(registry)?;
    let packages = registry
        .packages
        .iter()
        .map(|package| {
            let (support, causal, parity, future, semantic_alias) = receipt_digests
                .remove(&package.package_id)
                .ok_or(missing_receipts_error)?;
            let verifier = package.verifier.as_ref().ok_or(missing_verifier_error)?;
            Ok(nando_operator_admission::AdmissionPackageBindingInput {
                package_id: package.package_id.clone(),
                package_sha256: response_package_digest(package)?,
                execution_payload_sha256: response_execution_payload_digest(package)?,
                actor_program_sha256: response_actor_program_digest(&package.program)?,
                independent_verifier_program_sha256: response_independent_verifier_program_digest(
                    verifier,
                )?,
                verifier_schema: package.proof.verifier_schema.clone(),
                proof_roots: nando_operator_admission::AdmissionProofRoots {
                    support_manifest_sha256: support,
                    exact_causal_proof_sha256: causal,
                    runtime_parity_receipt_set_sha256: parity,
                    future_verifier_receipt_set_sha256: future,
                    semantic_alias_proof_sha256: semantic_alias,
                },
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    nando_operator_admission::build_composite_response_admission(
        project_id,
        &registry.schema,
        registry.revision,
        &registry_sha256,
        now_unix,
        max_age_seconds,
        gate_build_sha256,
        runtime_build_sha256,
        packages,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_response_authority(
    registry: &ResponseRegistry,
    admission: &CompositeResponseAdmissionV2,
    expected_project_id: &str,
    expected_gate_build_sha256: &str,
    expected_runtime_build_sha256: &str,
    now_unix: u64,
    max_age_seconds: u64,
) -> Result<ValidatedResponseAuthority, &'static str> {
    if registry.schema != RESPONSE_REGISTRY_SCHEMA_V6 {
        return Err("response_execution_requires_registry_v6");
    }
    registry.validate()?;
    let registry_sha256 = response_registry_digest(registry)?;
    let mut packages = registry
        .packages
        .iter()
        .filter(|package| package.eligible_for_admission_candidate())
        .map(|package| {
            let verifier = package
                .verifier
                .as_ref()
                .ok_or("response_authority_verifier_missing")?;
            let verifier_schema = response_program_external_verifier_schema(&package.program)
                .ok_or("response_authority_verifier_schema_missing")?;
            Ok(nando_operator_admission::AdmissionPackageRecord {
                package_id: package.package_id.clone(),
                package_sha256: response_package_digest(package)?,
                execution_payload_sha256: response_execution_payload_digest(package)?,
                actor_program_sha256: response_actor_program_digest(&package.program)?,
                independent_verifier_program_sha256: response_independent_verifier_program_digest(
                    verifier,
                )?,
                verifier_schema: verifier_schema.to_owned(),
                admission_candidate: true,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    nando_operator_admission::validate_response_authority_snapshot(
        &nando_operator_admission::AdmissionRegistrySnapshot {
            schema: registry.schema.clone(),
            revision: registry.revision,
            registry_sha256,
            packages,
        },
        admission,
        expected_project_id,
        expected_gate_build_sha256,
        expected_runtime_build_sha256,
        now_unix,
        max_age_seconds,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::json;

    use super::*;

    #[test]
    fn canonical_json_sorts_objects_and_rejects_floats() {
        assert_eq!(
            canonical_json_bytes(&json!({"z":2,"a":{"y":1,"b":true}})),
            Ok(br#"{"a":{"b":true,"y":1},"z":2}"#.to_vec())
        );
        assert_eq!(
            canonical_json_bytes(&json!({"float": 0.5})),
            Err("canonical_json_float_unsupported")
        );
    }

    #[test]
    fn post_verifier_receipt_matches_exact_economics_abi() {
        let actor = sha256_bytes(b"actor");
        let verifier = sha256_bytes(b"verifier");
        let request = sha256_bytes(b"request");
        let output = sha256_bytes(b"output");
        let finalized =
            finalize_post_verifier_receipt(&actor, &verifier, &request, &output, "package-v1")
                .expect("post-verifier receipt");
        let object = serde_json::to_value(&finalized.receipt)
            .expect("receipt json")
            .as_object()
            .expect("receipt object")
            .clone();
        assert_eq!(
            object.keys().cloned().collect::<BTreeSet<_>>(),
            [
                "actor_sha256",
                "admission_binding_sha256",
                "evidence_sha256",
                "output_sha256",
                "package_id_sha256",
                "schema",
                "verifier_sha256",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
        assert_eq!(finalized.receipt.evidence_sha256, request);
        assert_eq!(finalized.receipt.output_sha256, output);
        assert_eq!(
            finalized.receipt.package_id_sha256,
            sha256_bytes(b"package-v1")
        );
        assert_eq!(
            finalized.receipt_sha256,
            canonical_json_sha256(&finalized.receipt).expect("receipt digest")
        );
    }
}
