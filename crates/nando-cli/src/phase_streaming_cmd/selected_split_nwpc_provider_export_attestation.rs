use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{json_string, json_u64, read_json_value, write_json_file};

const DEFAULT_SELECTED_SPLIT_PROVIDER_EXPORT_ATTESTATION_CONTRACT_REPORT: &str = "target/nando-wave/streaming/phase-stream-selected-split-nwpc-provider-export-attestation-contract-v1.report.json";
pub(crate) const PROVIDER_EXPORT_ATTESTATION_SCHEMA_VERSION: &str =
    "provider_export_attestation_v1";
pub(crate) const PROVIDER_EXPORT_ATTESTATION_TEMPLATE_SCHEMA_VERSION: &str =
    "provider_export_attestation_template_v1";
pub(crate) const EXTERNAL_PROVIDER_BILLING_SOURCE_KIND: &str = "external_provider_billing_export";

#[derive(Clone, Serialize)]
pub(crate) struct ProviderExportAttestationReview {
    pub(crate) path: PathBuf,
    pub(crate) present: bool,
    pub(crate) valid: bool,
    pub(crate) blockers: Vec<String>,
}

pub(crate) fn run_phase_stream_selected_split_nwpc_provider_export_attestation_contract_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_SELECTED_SPLIT_PROVIDER_EXPORT_ATTESTATION_CONTRACT_REPORT)
    });
    let provider_export_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "provider export path is required".to_owned())?;
    let template_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| provider_export_attestation_template_path(&provider_export_path));
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument '{extra}'"));
    }

    let provider_export_fingerprint64 = provider_export_fingerprint64(&provider_export_path)?;
    let provider_export_line_count = provider_export_line_count(&provider_export_path)?;
    let required_attestation_path = provider_export_attestation_path(&provider_export_path);
    let existing_attestation_review =
        review_provider_export_attestation(&provider_export_path, provider_export_fingerprint64)?;

    let template = serde_json::json!({
        "schema_version": PROVIDER_EXPORT_ATTESTATION_TEMPLATE_SCHEMA_VERSION,
        "safe_to_use_as_evidence": false,
        "copy_required_attestation_to": required_attestation_path,
        "required_attestation": {
            "schema_version": PROVIDER_EXPORT_ATTESTATION_SCHEMA_VERSION,
            "source_kind": EXTERNAL_PROVIDER_BILLING_SOURCE_KIND,
            "provider": "replace-with-external-provider-name",
            "billing_source": "replace-with-external-provider-billing-export-name",
            "provider_export_fingerprint64": provider_export_fingerprint64,
            "captured_at": "replace-with-external-export-capture-time",
            "boundary": "attests the adjacent provider export file came from an external provider billing/usage export; does not by itself prove Nando savings"
        },
        "boundary": "template only: placeholders intentionally fail selected-split provider-export attestation validation until replaced with real external provider provenance"
    });
    write_json_file(&template_path, &template)?;

    let report = serde_json::json!({
        "report_kind": "phase_stream_selected_split_nwpc_provider_export_attestation_contract_v1",
        "provider_export_path": provider_export_path,
        "provider_export_line_count": provider_export_line_count,
        "provider_export_fingerprint64": provider_export_fingerprint64,
        "required_attestation_path": required_attestation_path,
        "template_path": template_path,
        "existing_attestation_review": existing_attestation_review,
        "local_accept_enabled": false,
        "auto_promote_enabled": false,
        "serving_registry_mutated": false,
        "product_runtime_changed": false,
        "serving_runtime_changed": false,
        "market_money_claim_allowed": false,
        "forbidden_flags": {
            "nwrb_used": false,
            "role_binding_backend_used": false,
            "lookup_used": false,
            "target_id_or_proof_rule_id_authority_used": false,
            "concrete_x_lookup_used": false,
            "manual_local_out_t_used": false,
            "manual_class_list_used": false,
            "manual_threshold_selection_used": false,
            "local_accept_without_verifier_used": false
        },
        "verdict": "PHASE_STREAM_SELECTED_SPLIT_NWPC_PROVIDER_EXPORT_ATTESTATION_CONTRACT_V1_READY",
        "boundary": "writes an attestation template/contract for a real external provider export; does not create valid evidence, does not enable local_accept, and does not allow a money claim"
    });
    write_json_file(&report_path, &report)?;
    println!("phase_stream_selected_split_nwpc_provider_export_attestation_contract_v1:");
    println!("  report_path: {}", report_path.display());
    println!("  template_path: {}", template_path.display());
    println!(
        "  required_attestation_path: {}",
        required_attestation_path.display()
    );
    println!("  provider_export_fingerprint64: {provider_export_fingerprint64}");
    println!("  market_money_claim_allowed: false");
    println!(
        "  verdict: PHASE_STREAM_SELECTED_SPLIT_NWPC_PROVIDER_EXPORT_ATTESTATION_CONTRACT_V1_READY"
    );
    Ok(())
}

pub(crate) fn provider_export_fingerprint64(provider_export_path: &Path) -> Result<u64, String> {
    let bytes = std::fs::read(provider_export_path).map_err(|error| {
        format!(
            "failed to read provider export '{}': {error}",
            provider_export_path.display()
        )
    })?;
    Ok(fnv1a64(&bytes))
}

pub(crate) fn review_provider_export_attestation(
    provider_export_path: &Path,
    provider_export_fingerprint64: u64,
) -> Result<ProviderExportAttestationReview, String> {
    let path = provider_export_attestation_path(provider_export_path);
    if !path.is_file() {
        return Ok(ProviderExportAttestationReview {
            path,
            present: false,
            valid: false,
            blockers: vec!["missing_provider_export_attestation".to_owned()],
        });
    }

    let attestation = read_json_value(&path)?;
    let mut blockers = Vec::new();
    if json_string(&attestation, &["schema_version"]).as_deref()
        != Some(PROVIDER_EXPORT_ATTESTATION_SCHEMA_VERSION)
    {
        blockers.push("invalid_attestation_schema_version".to_owned());
    }
    if json_string(&attestation, &["source_kind"]).as_deref()
        != Some(EXTERNAL_PROVIDER_BILLING_SOURCE_KIND)
    {
        blockers.push("invalid_attestation_source_kind".to_owned());
    }
    if json_string(&attestation, &["provider"]).is_none_or(|provider| {
        provider.trim().is_empty() || !external_billing_source_allowed(&provider)
    }) {
        blockers.push("invalid_attestation_provider".to_owned());
    }
    if json_string(&attestation, &["billing_source"])
        .is_none_or(|source| source.trim().is_empty() || !external_billing_source_allowed(&source))
    {
        blockers.push("invalid_attestation_billing_source".to_owned());
    }
    let attested_fingerprint = json_u64(&attestation, &["provider_export_fingerprint64"]);
    if attested_fingerprint != Some(provider_export_fingerprint64) {
        blockers.push("provider_export_fingerprint_mismatch".to_owned());
    }

    Ok(ProviderExportAttestationReview {
        path,
        present: true,
        valid: blockers.is_empty(),
        blockers,
    })
}

pub(crate) fn provider_export_attestation_path(provider_export_path: &Path) -> PathBuf {
    PathBuf::from(format!(
        "{}.attestation.json",
        provider_export_path.display()
    ))
}

fn provider_export_attestation_template_path(provider_export_path: &Path) -> PathBuf {
    PathBuf::from(format!(
        "{}.attestation.template.json",
        provider_export_path.display()
    ))
}

fn provider_export_line_count(provider_export_path: &Path) -> Result<usize, String> {
    let text = std::fs::read_to_string(provider_export_path).map_err(|error| {
        format!(
            "failed to read provider export '{}': {error}",
            provider_export_path.display()
        )
    })?;
    Ok(text.lines().filter(|line| !line.trim().is_empty()).count())
}

fn external_billing_source_allowed(source: &str) -> bool {
    let normalized = source.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    let forbidden = [
        "synthetic",
        "estimate",
        "estimated",
        "request",
        "generated",
        "internal",
        "debug",
        "test",
        "fixture",
        "user_approved",
        "price_config",
        "nando",
        "replace",
        "placeholder",
        "todo",
        "example",
        "sample",
        "template",
    ];
    !forbidden
        .iter()
        .any(|forbidden| normalized.contains(forbidden))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_provider_export_path(label: &str) -> PathBuf {
        let dir = PathBuf::from("target/nando-wave/test/provider-attestation");
        std::fs::create_dir_all(&dir).expect("test provider attestation dir");
        dir.join(format!("{label}-{}.provider.jsonl", std::process::id()))
    }

    fn write_json(path: &Path, value: serde_json::Value) {
        std::fs::write(
            path,
            format!(
                "{}\n",
                serde_json::to_string(&value).expect("serialize test JSON")
            ),
        )
        .expect("write test JSON");
    }

    #[test]
    fn provider_export_attestation_rejects_missing_template_and_mismatch() {
        let export_path = test_provider_export_path("missing-template-mismatch");
        std::fs::write(&export_path, "{\"provider\":\"openai\"}\n").expect("write provider export");
        let fingerprint = provider_export_fingerprint64(&export_path).expect("fingerprint");

        let missing =
            review_provider_export_attestation(&export_path, fingerprint).expect("missing review");
        assert!(!missing.present);
        assert!(!missing.valid);
        assert_eq!(
            missing.blockers,
            vec!["missing_provider_export_attestation"]
        );

        let attestation_path = provider_export_attestation_path(&export_path);
        write_json(
            &attestation_path,
            serde_json::json!({
                "schema_version": PROVIDER_EXPORT_ATTESTATION_SCHEMA_VERSION,
                "source_kind": EXTERNAL_PROVIDER_BILLING_SOURCE_KIND,
                "provider": "replace-with-external-provider-name",
                "billing_source": "sample provider billing template",
                "provider_export_fingerprint64": fingerprint
            }),
        );
        let template_review =
            review_provider_export_attestation(&export_path, fingerprint).expect("template review");
        assert!(template_review.present);
        assert!(!template_review.valid);
        assert!(
            template_review
                .blockers
                .contains(&"invalid_attestation_provider".to_owned())
        );
        assert!(
            template_review
                .blockers
                .contains(&"invalid_attestation_billing_source".to_owned())
        );

        write_json(
            &attestation_path,
            serde_json::json!({
                "schema_version": PROVIDER_EXPORT_ATTESTATION_SCHEMA_VERSION,
                "source_kind": EXTERNAL_PROVIDER_BILLING_SOURCE_KIND,
                "provider": "openai",
                "billing_source": "openai platform usage export 2026-07-07",
                "provider_export_fingerprint64": fingerprint.saturating_add(1)
            }),
        );
        let mismatch_review =
            review_provider_export_attestation(&export_path, fingerprint).expect("mismatch review");
        assert!(mismatch_review.present);
        assert!(!mismatch_review.valid);
        assert!(
            mismatch_review
                .blockers
                .contains(&"provider_export_fingerprint_mismatch".to_owned())
        );
    }

    #[test]
    fn provider_export_attestation_accepts_external_source_with_matching_fingerprint() {
        let export_path = test_provider_export_path("valid-external");
        std::fs::write(&export_path, "{\"provider\":\"openai\"}\n").expect("write provider export");
        let fingerprint = provider_export_fingerprint64(&export_path).expect("fingerprint");
        write_json(
            &provider_export_attestation_path(&export_path),
            serde_json::json!({
                "schema_version": PROVIDER_EXPORT_ATTESTATION_SCHEMA_VERSION,
                "source_kind": EXTERNAL_PROVIDER_BILLING_SOURCE_KIND,
                "provider": "openai",
                "billing_source": "openai platform usage export 2026-07-07",
                "provider_export_fingerprint64": fingerprint
            }),
        );

        let review =
            review_provider_export_attestation(&export_path, fingerprint).expect("valid review");

        assert!(review.present);
        assert!(review.valid);
        assert!(review.blockers.is_empty());
    }
}
