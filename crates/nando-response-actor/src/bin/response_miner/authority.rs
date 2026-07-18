use std::collections::{BTreeMap, BTreeSet};

use nando_response_actor::{
    GroundedWaveCausalReport, RESPONSE_AUTHORITY_SCHEMA_V2, RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2,
    RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2, RESPONSE_REGISTRY_SCHEMA_V6,
    RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1, RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1,
    RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1, ResponsePackage, ResponsePackageAuthorityBindingV2,
    ResponsePackageOrigin, ResponsePackageState, ResponseRegistry, ResponseSupportManifest,
    canonical_json_sha256, response_actor_program_digest, response_execution_payload_digest,
    response_independent_verifier_program_digest, response_package_digest,
    response_proof_receipts_digest, response_registry_digest, response_support_manifest_digest,
};
use serde_json::Value;

pub(super) fn aggregate_causal_verdict(
    package_ids: impl IntoIterator<Item = impl AsRef<str>>,
    reports: &BTreeMap<String, GroundedWaveCausalReport>,
) -> &'static str {
    let package_ids = package_ids
        .into_iter()
        .map(|package_id| package_id.as_ref().to_owned())
        .collect::<Vec<_>>();
    if package_ids.is_empty() {
        return "MISSING";
    }
    if package_ids.iter().all(|package_id| {
        reports
            .get(package_id)
            .is_some_and(|report| report.verdict == "PASS")
    }) {
        "PASS"
    } else {
        "WATCH"
    }
}

pub(super) fn compile_runtime_registry(
    revision: u64,
    packages: Vec<ResponsePackage>,
) -> ResponseRegistry {
    let mut packages = packages
        .into_iter()
        .filter(|package| package.origin == ResponsePackageOrigin::GroundedSynthesis)
        .collect::<Vec<_>>();
    let mut winner_by_execution = BTreeMap::<String, usize>::new();
    for (index, package) in packages.iter().enumerate() {
        if package.state != ResponsePackageState::Active {
            continue;
        }
        let fingerprint = canonical_json_sha256(&serde_json::json!({
            "program": package.program,
            "verifier": package.verifier,
            "routing_predicates": package.routing_predicates,
            "required_routing_atom_ids": package.required_routing_atom_ids,
            "phase_centers": package.phase_centers,
            "anti_centers": package.anti_centers,
            "wave_margin_micro": package.wave_margin_micro,
        }))
        .unwrap_or_else(|_| package.package_id.clone());
        winner_by_execution
            .entry(fingerprint)
            .and_modify(|winner| {
                let current = &packages[*winner];
                let candidate_score = (
                    package.proof.future_rows,
                    package.proof.support_rows,
                    std::cmp::Reverse(package.package_id.as_str()),
                );
                let current_score = (
                    current.proof.future_rows,
                    current.proof.support_rows,
                    std::cmp::Reverse(current.package_id.as_str()),
                );
                if candidate_score > current_score {
                    *winner = index;
                }
            })
            .or_insert(index);
    }
    let winners = winner_by_execution.into_values().collect::<BTreeSet<_>>();
    for (index, package) in packages.iter_mut().enumerate() {
        if package.state == ResponsePackageState::Active && !winners.contains(&index) {
            package.state = ResponsePackageState::Revoked;
        }
    }
    ResponseRegistry {
        schema: RESPONSE_REGISTRY_SCHEMA_V6.to_owned(),
        revision,
        packages,
    }
}

pub(super) fn package_receipt_sets(
    revision: u64,
    receipts: &[Value],
    package_schema: &str,
) -> Vec<Value> {
    let mut by_package = BTreeMap::<String, Vec<Value>>::new();
    for receipt in receipts {
        let Some(package_id) = receipt.get("package_id").and_then(Value::as_str) else {
            continue;
        };
        by_package
            .entry(package_id.to_owned())
            .or_default()
            .push(receipt.clone());
    }
    by_package
        .into_iter()
        .map(|(package_id, mut receipts)| {
            receipts.sort_by(|left, right| {
                left.get("frame_id_sha256")
                    .and_then(Value::as_str)
                    .cmp(&right.get("frame_id_sha256").and_then(Value::as_str))
            });
            serde_json::json!({
                "schema": package_schema,
                "package_id": package_id,
                "registry_revision": revision,
                "receipts": receipts,
            })
        })
        .collect()
}

pub(super) fn response_authority_candidate(
    registry: &ResponseRegistry,
    manifests: &[ResponseSupportManifest],
    causal_reports: &BTreeMap<String, GroundedWaveCausalReport>,
    future_receipt_packages: &[Value],
    parity_receipt_packages: &[Value],
) -> Result<Value, String> {
    let registry_sha256 = response_registry_digest(registry).map_err(str::to_owned)?;
    let mut bindings = Vec::new();
    for package in registry
        .packages
        .iter()
        .filter(|package| package.eligible_for_admission_candidate())
    {
        let manifest = manifests
            .iter()
            .find(|manifest| manifest.package_id == package.package_id)
            .ok_or_else(|| format!("authority_manifest_missing:{}", package.package_id))?;
        let expected_manifest_sha256 =
            response_support_manifest_digest(manifest).map_err(str::to_owned)?;
        if manifest.schema != RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1
            || manifest.manifest_sha256 != expected_manifest_sha256
        {
            return Err(format!(
                "authority_manifest_digest_mismatch:{}",
                package.package_id
            ));
        }
        let causal = causal_reports
            .get(&package.package_id)
            .ok_or_else(|| format!("authority_causal_missing:{}", package.package_id))?;
        if causal.schema != RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2 || causal.verdict != "PASS" {
            return Err(format!("authority_causal_not_pass:{}", package.package_id));
        }
        let future_receipts = receipt_package(future_receipt_packages, &package.package_id)
            .ok_or_else(|| format!("authority_future_receipts_missing:{}", package.package_id))?;
        let parity_receipts = receipt_package(parity_receipt_packages, &package.package_id)
            .ok_or_else(|| format!("authority_parity_receipts_missing:{}", package.package_id))?;
        let verifier = package
            .verifier
            .as_ref()
            .ok_or_else(|| format!("authority_verifier_missing:{}", package.package_id))?;
        let mut binding = ResponsePackageAuthorityBindingV2 {
            package_id: package.package_id.clone(),
            registry_revision: registry.revision,
            package_sha256: response_package_digest(package).map_err(str::to_owned)?,
            execution_payload_sha256: response_execution_payload_digest(package)
                .map_err(str::to_owned)?,
            actor_program_sha256: response_actor_program_digest(&package.program)
                .map_err(str::to_owned)?,
            independent_verifier_program_sha256: response_independent_verifier_program_digest(
                verifier,
            )
            .map_err(str::to_owned)?,
            verifier_schema: package.proof.verifier_schema.clone(),
            support_manifest_schema: RESPONSE_SUPPORT_MANIFEST_SCHEMA_V1.to_owned(),
            support_manifest_sha256: manifest.manifest_sha256.clone(),
            exact_causal_proof_schema: RESPONSE_EXACT_CAUSAL_PROOF_SCHEMA_V2.to_owned(),
            exact_causal_proof_sha256: canonical_json_sha256(causal).map_err(str::to_owned)?,
            runtime_parity_receipt_set_schema: RESPONSE_RUNTIME_PARITY_RECEIPT_SET_SCHEMA_V1
                .to_owned(),
            runtime_parity_receipt_set_sha256: canonical_json_sha256(parity_receipts)
                .map_err(str::to_owned)?,
            future_verifier_receipt_set_schema: RESPONSE_FUTURE_VERIFIER_RECEIPT_SET_SCHEMA_V2
                .to_owned(),
            future_verifier_receipt_set_sha256: canonical_json_sha256(future_receipts)
                .map_err(str::to_owned)?,
            semantic_alias_proof_schema: RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1.to_owned(),
            semantic_alias_proof_sha256: canonical_json_sha256(&(
                RESPONSE_SEMANTIC_ALIAS_PROOF_SCHEMA_V1,
                "exact_singleton",
                package.package_id.as_str(),
                &package.program,
                causal,
            ))
            .map_err(str::to_owned)?,
            proof_receipts_sha256: String::new(),
        };
        binding.proof_receipts_sha256 =
            response_proof_receipts_digest(&binding).map_err(str::to_owned)?;
        bindings.push(binding);
    }
    bindings.sort_by(|left, right| left.package_id.cmp(&right.package_id));
    Ok(serde_json::json!({
        "schema": "nando.response-authority-candidate.v1",
        "authority_schema": RESPONSE_AUTHORITY_SCHEMA_V2,
        "registry_schema": registry.schema,
        "registry_revision": registry.revision,
        "registry_sha256": registry_sha256,
        "packages": bindings,
        "required_gate_fields": [
            "gate_build_sha256",
            "runtime_build_sha256",
            "generated_at_unix",
            "expires_at_unix",
        ],
        "execution_authority": false,
    }))
}

fn receipt_package<'a>(packages: &'a [Value], package_id: &str) -> Option<&'a Value> {
    packages.iter().find(|package| {
        package.get("package_id").and_then(Value::as_str) == Some(package_id)
            && package
                .get("registry_revision")
                .and_then(Value::as_u64)
                .is_some()
    })
}
