use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    ResponsePackage, ResponseProgram, ResponseRegistry, ResponseRoutingPredicate, VerifierProgram,
    response_program_external_verifier_schema,
};

pub const RESPONSE_REGISTRY_SCHEMA_V6: &str = "nando.response-registry.v6";
pub const RESPONSE_EXECUTION_PAYLOAD_SCHEMA_V1: &str = "nando.response-execution-payload.v1";
pub const COMPOSITE_ADMISSION_SCHEMA_V2: &str = "nando.live-transition-composite-gate.v2";
pub const RESPONSE_AUTHORITY_SCHEMA_V2: &str = "nando.response-authority.v2";
pub const RESPONSE_RUNTIME_RECEIPT_SCHEMA_V2: &str =
    "nando.response-runtime-verification-receipt.v2";
pub const RESPONSE_POST_VERIFIER_RECEIPT_SCHEMA_V1: &str =
    "nando.response-post-verifier-receipt.v1";
pub const RESPONSE_POST_VERIFIER_ADMISSION_SCHEMA_V1: &str = "nando.response-admission-binding.v1";
pub const RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1: &str = "nando.response-support-manifest.v1";
pub const RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2: &str =
    "nando.grounded-response-wave-causal-report.v2";
pub const RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1: &str =
    "nando.response-runtime-parity-receipt-set.v1";
pub const RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2: &str =
    "nando.response-future-verifier-receipt-set.v2";
pub const RESPONSE_FUTURE_VERIFIER_RECEIPT_SCHEMA_V2: &str =
    "nando.response-future-verifier-receipt.v2";
pub const RESPONSE_PROOF_RECEIPT_BINDING_SCHEMA_V1: &str =
    "nando.response-proof-receipt-binding.v1";
pub const RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1: &str = "nando.semantic-alias-proof.v1";
pub const RESPONSE_RUNTIME_CONTRACT_SCHEMA_V1: &str = "nando.response-runtime-contract.v1";

const MAX_ADMISSION_FUTURE_SKEW_SECONDS: u64 = 5;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompositeResponseAdmissionV2 {
    pub schema: String,
    pub project_id: String,
    pub generated_at_unix: u64,
    pub expires_at_unix: u64,
    pub verdict: String,
    pub eligible_for_local_accept: bool,
    pub response_authority: ResponseAuthorityV2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseAuthorityV2 {
    pub schema: String,
    pub registry_schema: String,
    pub registry_revision: u64,
    pub registry_sha256: String,
    pub gate_build_sha256: String,
    pub runtime_build_sha256: String,
    pub packages: Vec<ResponsePackageAuthorityBindingV2>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResponsePackageAuthorityBindingV2 {
    pub package_id: String,
    pub registry_revision: u64,
    pub package_sha256: String,
    pub execution_payload_sha256: String,
    pub actor_program_sha256: String,
    pub independent_verifier_program_sha256: String,
    pub verifier_schema: String,
    pub support_manifest_schema: String,
    pub support_manifest_sha256: String,
    pub exact_causal_proof_schema: String,
    pub exact_causal_proof_sha256: String,
    pub runtime_parity_receipt_set_schema: String,
    pub runtime_parity_receipt_set_sha256: String,
    pub future_verifier_receipt_set_schema: String,
    pub future_verifier_receipt_set_sha256: String,
    #[serde(default)]
    pub semantic_alias_proof_schema: String,
    #[serde(default)]
    pub semantic_alias_proof_sha256: String,
    pub proof_receipts_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AuthorizedResponsePackage {
    pub admission_sha256: String,
    pub registry_sha256: String,
    pub registry_revision: u64,
    pub package_sha256: String,
    pub execution_payload_sha256: String,
    pub actor_program_sha256: String,
    pub independent_verifier_program_sha256: String,
    pub verifier_schema: String,
    pub gate_build_sha256: String,
    pub runtime_build_sha256: String,
    pub support_manifest_sha256: String,
    pub exact_causal_proof_sha256: String,
    pub runtime_parity_receipt_set_sha256: String,
    pub future_verifier_receipt_set_sha256: String,
    pub semantic_alias_proof_sha256: String,
    pub proof_receipts_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ValidatedResponseAuthority {
    pub admission_sha256: String,
    pub registry_sha256: String,
    pub packages: BTreeMap<String, AuthorizedResponsePackage>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IndependentlyVerifiedExecution {
    pub package_id: String,
    pub authority: AuthorizedResponsePackage,
    pub provider_evidence_sha256: String,
    pub actor_output_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeVerificationResultV2 {
    Pass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeVerificationReceiptV2 {
    pub schema: String,
    pub admission_sha256: String,
    pub registry_sha256: String,
    pub registry_revision: u64,
    pub package_id: String,
    pub package_sha256: String,
    pub execution_payload_sha256: String,
    pub actor_program_sha256: String,
    pub independent_verifier_program_sha256: String,
    pub verifier_schema: String,
    pub support_manifest_sha256: String,
    pub exact_causal_proof_sha256: String,
    pub runtime_parity_receipt_set_sha256: String,
    pub future_verifier_receipt_set_sha256: String,
    pub semantic_alias_proof_sha256: String,
    pub proof_receipts_sha256: String,
    pub gate_build_sha256: String,
    pub runtime_build_sha256: String,
    pub projector_schema: String,
    pub projector_program_sha256: String,
    pub request_sha256: String,
    pub provider_evidence_sha256: String,
    pub actor_output_sha256: String,
    pub projected_output_sha256: String,
    pub result: RuntimeVerificationResultV2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedRuntimeVerificationReceiptV2 {
    pub receipt_sha256: String,
    pub receipt: RuntimeVerificationReceiptV2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PostVerifierReceiptV1 {
    pub schema: String,
    pub actor_sha256: String,
    pub verifier_sha256: String,
    pub evidence_sha256: String,
    pub output_sha256: String,
    pub package_id_sha256: String,
    pub admission_binding_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedPostVerifierReceiptV1 {
    pub receipt_sha256: String,
    pub receipt: PostVerifierReceiptV1,
}

#[derive(Serialize)]
struct PostVerifierAdmissionBindingMaterial<'a> {
    schema: &'static str,
    actor_sha256: &'a str,
    verifier_sha256: &'a str,
    package_id_sha256: &'a str,
}

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

#[derive(Serialize)]
struct ProofReceiptBindingDigestMaterial<'a> {
    schema: &'static str,
    package_id: &'a str,
    registry_revision: u64,
    support_manifest_schema: &'a str,
    support_manifest_sha256: &'a str,
    exact_causal_proof_schema: &'a str,
    exact_causal_proof_sha256: &'a str,
    runtime_parity_receipt_set_schema: &'a str,
    runtime_parity_receipt_set_sha256: &'a str,
    future_verifier_receipt_set_schema: &'a str,
    future_verifier_receipt_set_sha256: &'a str,
    semantic_alias_proof_schema: &'a str,
    semantic_alias_proof_sha256: &'a str,
}

pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, &'static str> {
    let value = serde_json::to_value(value).map_err(|_| "canonical_json_serialize_failed")?;
    let mut output = Vec::new();
    write_canonical_json(&value, &mut output)?;
    Ok(output)
}

pub fn canonical_json_sha256<T: Serialize>(value: &T) -> Result<String, &'static str> {
    Ok(sha256_bytes(&canonical_json_bytes(value)?))
}

#[must_use]
pub fn sha256_bytes(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
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

#[must_use]
pub fn valid_nonzero_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
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

pub fn response_proof_receipts_digest(
    binding: &ResponsePackageAuthorityBindingV2,
) -> Result<String, &'static str> {
    canonical_json_sha256(&ProofReceiptBindingDigestMaterial {
        schema: RESPONSE_PROOF_RECEIPT_BINDING_SCHEMA_V1,
        package_id: &binding.package_id,
        registry_revision: binding.registry_revision,
        support_manifest_schema: &binding.support_manifest_schema,
        support_manifest_sha256: &binding.support_manifest_sha256,
        exact_causal_proof_schema: &binding.exact_causal_proof_schema,
        exact_causal_proof_sha256: &binding.exact_causal_proof_sha256,
        runtime_parity_receipt_set_schema: &binding.runtime_parity_receipt_set_schema,
        runtime_parity_receipt_set_sha256: &binding.runtime_parity_receipt_set_sha256,
        future_verifier_receipt_set_schema: &binding.future_verifier_receipt_set_schema,
        future_verifier_receipt_set_sha256: &binding.future_verifier_receipt_set_sha256,
        semantic_alias_proof_schema: &binding.semantic_alias_proof_schema,
        semantic_alias_proof_sha256: &binding.semantic_alias_proof_sha256,
    })
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
    if admission.schema != COMPOSITE_ADMISSION_SCHEMA_V2 {
        return Err("response_admission_schema_not_v2");
    }
    if admission.project_id != expected_project_id {
        return Err("response_admission_foreign_project");
    }
    if admission.verdict != "PASS" || !admission.eligible_for_local_accept {
        return Err("response_admission_not_pass");
    }
    if admission.generated_at_unix > now_unix.saturating_add(MAX_ADMISSION_FUTURE_SKEW_SECONDS) {
        return Err("response_admission_from_future");
    }
    if now_unix.saturating_sub(admission.generated_at_unix) > max_age_seconds {
        return Err("response_admission_stale");
    }
    if admission.expires_at_unix < admission.generated_at_unix
        || admission.expires_at_unix > admission.generated_at_unix.saturating_add(max_age_seconds)
        || now_unix > admission.expires_at_unix
    {
        return Err("response_admission_expired");
    }
    let authority = &admission.response_authority;
    if authority.schema != RESPONSE_AUTHORITY_SCHEMA_V2 {
        return Err("response_authority_schema_not_v2");
    }
    if authority.registry_schema != registry.schema {
        return Err("response_authority_registry_schema_mismatch");
    }
    if authority.registry_revision != registry.revision {
        return Err("response_authority_registry_revision_mismatch");
    }
    let registry_sha256 = response_registry_digest(registry)?;
    if authority.registry_sha256 != registry_sha256 {
        return Err("response_authority_registry_digest_mismatch");
    }
    if !valid_nonzero_sha256(expected_gate_build_sha256)
        || authority.gate_build_sha256 != expected_gate_build_sha256
    {
        return Err("response_authority_gate_build_mismatch");
    }
    if !valid_nonzero_sha256(expected_runtime_build_sha256)
        || authority.runtime_build_sha256 != expected_runtime_build_sha256
    {
        return Err("response_authority_runtime_build_mismatch");
    }
    if authority.packages.is_empty()
        || authority
            .packages
            .windows(2)
            .any(|pair| pair[0].package_id >= pair[1].package_id)
    {
        return Err("response_authority_packages_not_strictly_sorted");
    }
    let package_by_id = registry
        .packages
        .iter()
        .map(|package| (package.package_id.as_str(), package))
        .collect::<BTreeMap<_, _>>();
    let candidate_ids = registry
        .packages
        .iter()
        .filter(|package| package.eligible_for_admission_candidate())
        .map(|package| package.package_id.as_str())
        .collect::<BTreeSet<_>>();
    let admitted_ids = authority
        .packages
        .iter()
        .map(|binding| binding.package_id.as_str())
        .collect::<BTreeSet<_>>();
    if candidate_ids != admitted_ids {
        return Err("response_authority_candidate_set_mismatch");
    }
    let admission_sha256 = canonical_json_sha256(admission)?;
    let mut validated = BTreeMap::new();
    for binding in &authority.packages {
        let package = package_by_id
            .get(binding.package_id.as_str())
            .copied()
            .ok_or("response_authority_package_missing")?;
        validate_binding(package, registry.revision, binding)?;
        validated.insert(
            binding.package_id.clone(),
            AuthorizedResponsePackage {
                admission_sha256: admission_sha256.clone(),
                registry_sha256: registry_sha256.clone(),
                registry_revision: registry.revision,
                package_sha256: binding.package_sha256.clone(),
                execution_payload_sha256: binding.execution_payload_sha256.clone(),
                actor_program_sha256: binding.actor_program_sha256.clone(),
                independent_verifier_program_sha256: binding
                    .independent_verifier_program_sha256
                    .clone(),
                verifier_schema: binding.verifier_schema.clone(),
                gate_build_sha256: authority.gate_build_sha256.clone(),
                runtime_build_sha256: authority.runtime_build_sha256.clone(),
                support_manifest_sha256: binding.support_manifest_sha256.clone(),
                exact_causal_proof_sha256: binding.exact_causal_proof_sha256.clone(),
                runtime_parity_receipt_set_sha256: binding
                    .runtime_parity_receipt_set_sha256
                    .clone(),
                future_verifier_receipt_set_sha256: binding
                    .future_verifier_receipt_set_sha256
                    .clone(),
                semantic_alias_proof_sha256: binding.semantic_alias_proof_sha256.clone(),
                proof_receipts_sha256: binding.proof_receipts_sha256.clone(),
            },
        );
    }
    Ok(ValidatedResponseAuthority {
        admission_sha256,
        registry_sha256,
        packages: validated,
    })
}

pub(crate) fn finalize_runtime_receipt(
    verified: &IndependentlyVerifiedExecution,
    request_sha256: &str,
    projector_schema: &str,
    projector_program_sha256: &str,
    projected_output: &Value,
) -> Result<FinalizedRuntimeVerificationReceiptV2, &'static str> {
    if !valid_nonzero_sha256(request_sha256) {
        return Err("runtime_receipt_request_digest_invalid");
    }
    if projector_schema.is_empty() || !valid_nonzero_sha256(projector_program_sha256) {
        return Err("runtime_receipt_projector_binding_invalid");
    }
    let receipt = RuntimeVerificationReceiptV2 {
        schema: RESPONSE_RUNTIME_RECEIPT_SCHEMA_V2.to_owned(),
        admission_sha256: verified.authority.admission_sha256.clone(),
        registry_sha256: verified.authority.registry_sha256.clone(),
        registry_revision: verified.authority.registry_revision,
        package_id: verified.package_id.clone(),
        package_sha256: verified.authority.package_sha256.clone(),
        execution_payload_sha256: verified.authority.execution_payload_sha256.clone(),
        actor_program_sha256: verified.authority.actor_program_sha256.clone(),
        independent_verifier_program_sha256: verified
            .authority
            .independent_verifier_program_sha256
            .clone(),
        verifier_schema: verified.authority.verifier_schema.clone(),
        support_manifest_sha256: verified.authority.support_manifest_sha256.clone(),
        exact_causal_proof_sha256: verified.authority.exact_causal_proof_sha256.clone(),
        runtime_parity_receipt_set_sha256: verified
            .authority
            .runtime_parity_receipt_set_sha256
            .clone(),
        future_verifier_receipt_set_sha256: verified
            .authority
            .future_verifier_receipt_set_sha256
            .clone(),
        semantic_alias_proof_sha256: verified.authority.semantic_alias_proof_sha256.clone(),
        proof_receipts_sha256: verified.authority.proof_receipts_sha256.clone(),
        gate_build_sha256: verified.authority.gate_build_sha256.clone(),
        runtime_build_sha256: verified.authority.runtime_build_sha256.clone(),
        projector_schema: projector_schema.to_owned(),
        projector_program_sha256: projector_program_sha256.to_owned(),
        request_sha256: request_sha256.to_owned(),
        provider_evidence_sha256: verified.provider_evidence_sha256.clone(),
        actor_output_sha256: verified.actor_output_sha256.clone(),
        projected_output_sha256: canonical_json_sha256(projected_output)?,
        result: RuntimeVerificationResultV2::Pass,
    };
    let receipt_sha256 = canonical_json_sha256(&receipt)?;
    Ok(FinalizedRuntimeVerificationReceiptV2 {
        receipt_sha256,
        receipt,
    })
}

pub fn finalize_post_verifier_receipt(
    actor_sha256: &str,
    verifier_sha256: &str,
    request_sha256: &str,
    projector_receipt_id: &str,
    package_id: &str,
) -> Result<FinalizedPostVerifierReceiptV1, &'static str> {
    if !valid_nonzero_sha256(actor_sha256)
        || !valid_nonzero_sha256(verifier_sha256)
        || !valid_nonzero_sha256(request_sha256)
        || !valid_nonzero_sha256(projector_receipt_id)
        || package_id.is_empty()
        || package_id.len() > 256
    {
        return Err("post_verifier_receipt_binding_invalid");
    }
    let package_id_sha256 = sha256_bytes(package_id.as_bytes());
    let admission_binding_sha256 = canonical_json_sha256(&PostVerifierAdmissionBindingMaterial {
        schema: RESPONSE_POST_VERIFIER_ADMISSION_SCHEMA_V1,
        actor_sha256,
        verifier_sha256,
        package_id_sha256: &package_id_sha256,
    })?;
    let receipt = PostVerifierReceiptV1 {
        schema: RESPONSE_POST_VERIFIER_RECEIPT_SCHEMA_V1.to_owned(),
        actor_sha256: actor_sha256.to_owned(),
        verifier_sha256: verifier_sha256.to_owned(),
        evidence_sha256: request_sha256.to_owned(),
        output_sha256: projector_receipt_id.to_owned(),
        package_id_sha256,
        admission_binding_sha256,
    };
    let receipt_sha256 = canonical_json_sha256(&receipt)?;
    Ok(FinalizedPostVerifierReceiptV1 {
        receipt_sha256,
        receipt,
    })
}

fn validate_binding(
    package: &ResponsePackage,
    registry_revision: u64,
    binding: &ResponsePackageAuthorityBindingV2,
) -> Result<(), &'static str> {
    if !package.eligible_for_admission_candidate() {
        return Err("response_authority_package_not_candidate");
    }
    if binding.registry_revision != registry_revision {
        return Err("response_authority_package_revision_mismatch");
    }
    if response_package_digest(package)? != binding.package_sha256 {
        return Err("response_authority_package_digest_mismatch");
    }
    if response_execution_payload_digest(package)? != binding.execution_payload_sha256 {
        return Err("response_authority_payload_digest_mismatch");
    }
    if response_actor_program_digest(&package.program)? != binding.actor_program_sha256 {
        return Err("response_authority_actor_digest_mismatch");
    }
    let verifier = package
        .verifier
        .as_ref()
        .ok_or("response_authority_verifier_missing")?;
    if response_independent_verifier_program_digest(verifier)?
        != binding.independent_verifier_program_sha256
    {
        return Err("response_authority_verifier_digest_mismatch");
    }
    let expected_verifier_schema = response_program_external_verifier_schema(&package.program)
        .ok_or("response_authority_verifier_schema_missing")?;
    if package.proof.verifier_schema != expected_verifier_schema
        || binding.verifier_schema != expected_verifier_schema
    {
        return Err("response_authority_verifier_schema_mismatch");
    }
    validate_external_receipt(
        &binding.support_manifest_schema,
        RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1,
        &binding.support_manifest_sha256,
        "response_authority_support_manifest_invalid",
    )?;
    validate_external_receipt(
        &binding.exact_causal_proof_schema,
        RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2,
        &binding.exact_causal_proof_sha256,
        "response_authority_exact_causal_proof_invalid",
    )?;
    validate_external_receipt(
        &binding.runtime_parity_receipt_set_schema,
        RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1,
        &binding.runtime_parity_receipt_set_sha256,
        "response_authority_runtime_parity_receipt_invalid",
    )?;
    validate_external_receipt(
        &binding.future_verifier_receipt_set_schema,
        RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2,
        &binding.future_verifier_receipt_set_sha256,
        "response_authority_future_verifier_receipt_invalid",
    )?;
    validate_external_receipt(
        &binding.semantic_alias_proof_schema,
        RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1,
        &binding.semantic_alias_proof_sha256,
        "response_authority_semantic_alias_proof_invalid",
    )?;
    if response_proof_receipts_digest(binding)? != binding.proof_receipts_sha256 {
        return Err("response_authority_proof_receipts_digest_mismatch");
    }
    Ok(())
}

fn validate_external_receipt(
    actual_schema: &str,
    expected_schema: &str,
    digest: &str,
    error: &'static str,
) -> Result<(), &'static str> {
    if actual_schema != expected_schema || !valid_nonzero_sha256(digest) {
        return Err(error);
    }
    Ok(())
}

fn write_canonical_json(value: &Value, output: &mut Vec<u8>) -> Result<(), &'static str> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(true) => output.extend_from_slice(b"true"),
        Value::Bool(false) => output.extend_from_slice(b"false"),
        Value::Number(number) => {
            if number.as_i64().is_none() && number.as_u64().is_none() {
                return Err("canonical_json_float_unsupported");
            }
            output.extend_from_slice(number.to_string().as_bytes());
        }
        Value::String(string) => output.extend_from_slice(
            serde_json::to_string(string)
                .map_err(|_| "canonical_json_string_failed")?
                .as_bytes(),
        ),
        Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                write_canonical_json(value, output)?;
            }
            output.push(b']');
        }
        Value::Object(values) => {
            output.push(b'{');
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                output.extend_from_slice(
                    serde_json::to_string(key)
                        .map_err(|_| "canonical_json_key_failed")?
                        .as_bytes(),
                );
                output.push(b':');
                write_canonical_json(&values[key], output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
